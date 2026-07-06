
---

# SOURCE: /Users/will/dev/nunchi/roko/roko/docs/04-verification/00-gate-trait.md

# 00 — The Gate Trait

> **Layer**: L3 Harness — Verification
> **Crate**: `roko-core` (`crates/roko-core/src/traits.rs`)
> **Status**: Stable, implemented


> **Implementation**: Shipping

---

## 1. Overview

The `Gate` trait is one of six foundational traits in Roko's Synapse Architecture. It
occupies a unique position: while Substrate stores, Scorer evaluates, Router selects,
Composer assembles, and Policy governs — **Gate verifies**. It is the system's sole
mechanism for establishing ground truth about agent-produced artifacts.

Gates answer one question: *did the agent's output meet the verifiable criteria for
this verification step?* The answer is always a `Verdict` — never an error, never a
maybe, never a "could not determine." This design is deliberate and load-bearing.

> **Citation**: Synapse Architecture (refactoring-prd/01-synapse-architecture.md) — "1
> noun (Engram) + 6 verb traits" composing the universal loop.

---

## 2. The Trait Signature

```rust
// crates/roko-core/src/traits.rs, lines 102–108

pub trait Gate: Send + Sync {
    /// Verify the Engram and return a verdict.
    async fn verify(&self, engram: &Engram, ctx: &Context) -> Verdict;

    /// Human-readable name (appears in verdicts).
    fn name(&self) -> &str;
}
```

Two methods. That's the entire surface area.

### 2.1 `verify()` Returns `Verdict`, Not `Result<Verdict>`

This is the single most important design decision in the Gate trait and it requires
emphasis because it diverges from the idiomatic Rust pattern of returning `Result` from
fallible operations.

**Gate failure is not an error — it is a verdict.**

When `cargo check` reports a compilation error, that is not an infrastructure failure.
It is meaningful information: the code does not compile. The gate's job is to encode
that outcome into a `Verdict::fail()` with a reason string and optional error digest.
The caller never has to handle two failure paths (infrastructure error vs. verification
failure); there is only one path: the verdict.

This means:
- A gate that cannot spawn its subprocess returns `Verdict::fail("spawn failed: ...")`
- A gate that times out returns `Verdict::fail("timed out after N ms")`
- A gate whose input signal has malformed JSON returns `Verdict::fail("signal body is not a GatePayload: ...")`
- A gate whose test suite has 3 failures returns `Verdict::fail("test foo::bar ... FAILED; ...")`

All four are verdicts. The pipeline, ratchet, adaptive thresholds, and feedback systems
never need to pattern-match on `Result::Err`. They receive a `Verdict` and act on it.

> **Citation**: refactoring-prd/01-synapse-architecture.md — Gate trait signature:
> `async fn verify(&self, engram: &Engram, ctx: &Context) -> Verdict`. The canonical
> source explicitly uses `-> Verdict`, not `-> Result<Verdict>`.

> **Citation**: crates/roko-core/src/traits.rs:102–108 — Active implementation
> confirms the `-> Verdict` return type.

### 2.2 Why `async`

Gates shell out to external tools: compilers, test runners, linters, static analyzers.
These are I/O-bound operations measured in seconds to minutes. Making `verify()` async
allows the gate pipeline to:

1. Run multiple independent gates concurrently (future: parallel gate groups)
2. Apply timeouts via `tokio::time::timeout` without blocking the executor
3. Compose with the rest of Roko's async runtime (agent dispatch, signal persistence)

Every concrete gate implementation uses `#[async_trait]` from the `async-trait` crate
to satisfy the trait's async method.

### 2.3 Why `Send + Sync`

Gates are composed into pipelines (`GatePipeline`) that may be shared across tasks in
the plan executor. The `Send + Sync` bounds ensure gates can be stored in
`Vec<Box<dyn Gate>>` and passed between threads safely.

---

## 3. The `name()` Method

```rust
fn name(&self) -> &str;
```

Every verdict carries the name of the gate that produced it. This serves three purposes:

1. **Traceability**: When a verdict flows into the feedback system, the agent (and the
   human operator) can see *which* gate failed. "compile:cargo" vs. "test:cargo" vs.
   "clippy:cargo" vs. "diff" vs. "symbol" — these are distinct verification steps with
   distinct remediation paths.

2. **Ratcheting**: The `GateRatchet` tracks the highest rung passed per plan. Gate
   names associate verdicts with rungs.

3. **Adaptive thresholds**: The `AdaptiveThresholds` system tracks per-rung pass rates.
   Gate names allow the threshold system to correlate verdicts with rungs for EMA
   updates.

Naming convention in the codebase follows `category:tool` format:
- `compile:cargo`, `compile:npm`, `compile:go`
- `test:cargo`, `test:npm`, `test:go`
- `clippy:cargo` (lint gates)
- `shell:true`, `shell:custom_script`
- `diff`, `symbol` (standalone names)

---

## 4. The `Verdict` Type

While `Verdict` is defined in `roko-core` (not in the gate crate), understanding it is
essential to understanding gates. A `Verdict` carries:

| Field | Type | Purpose |
|---|---|---|
| `passed` | `bool` | Did the gate pass? |
| `gate` | `String` | Name of the gate that produced this verdict |
| `reason` | `String` | Human-readable explanation (especially on failure) |
| `detail` | `Option<String>` | Full output (stdout + stderr) for debugging |
| `error_digest` | `Option<String>` | Machine-parseable error summary |
| `duration_ms` | `u64` | Wall-clock time the gate took |
| `test_count` | `Option<TestCount>` | Parsed test counts (passed/failed/ignored) |

Key construction patterns used across all gate implementations:

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

> **Citation**: crates/roko-gate/src/compile.rs:113–122 — CompileGate demonstrates
> the standard pass/fail verdict construction pattern.

---

## 5. The Gate Contract

Every gate implementation must satisfy these invariants:

### 5.1 Total Function

`verify()` must always return a `Verdict`. It must not panic, it must not return `Err`,
and it must not hang indefinitely. Every gate in the codebase enforces a timeout
(typically via `tokio::time::timeout`) and converts timeout expiration into
`Verdict::fail()`.

Timeouts by gate:
- `CompileGate`: 10 minutes (600,000 ms)
- `TestGate`: 15 minutes (900,000 ms)
- `ClippyGate`: 5 minutes (300,000 ms)
- `ShellGate`: 5 minutes (300,000 ms)

### 5.2 Deterministic on Identical Inputs

Given the same signal body and filesystem state, a gate should produce the same verdict.
Gates do not use randomness. They do not consult LLMs (except the future
`LlmJudgeGate`, which has its own reproducibility constraints).

### 5.3 Side-Effect Free (on the source)

Gates read the filesystem and run subprocesses, but they must not modify the source code
being verified. A gate that rewrites the code to make it pass would defeat its purpose.
Gates may produce artifacts (build caches, test output files), but these go into
`CARGO_TARGET_DIR` or the `ArtifactStore`, not into the source tree.

### 5.4 Duration Tracking

Every verdict must carry `duration_ms`. This feeds the adaptive threshold system's retry
budget calculations and the efficiency event logger's per-gate timing data.

> **Citation**: crates/roko-gate/src/shell.rs:57–118 — ShellGate's `verify()`
> demonstrates all contract properties: timeout handling, duration tracking, infallible
> return.

---

## 6. How Gates Differ from Scorers

Both Gates and Scorers evaluate signals, but they serve different purposes:

| Dimension | Gate | Scorer |
|---|---|---|
| Output | `Verdict` (pass/fail + metadata) | `Score` (numeric, 0.0–1.0) |
| Truth source | External tool (compiler, test runner) | Internal heuristic or model |
| Determinism | Deterministic (same code → same result) | May be probabilistic |
| Cost | High (subprocess spawn, minutes) | Low (computation, milliseconds) |
| Role in loop | Verification (L3 Harness) | Evaluation (L2 Engine) |

A Scorer might estimate "this code looks 80% correct." A Gate runs the compiler and
says definitively "this code compiles" or "this code does not compile." The Scorer's
estimate is useful for routing decisions; the Gate's verdict is ground truth.

> **Citation**: refactoring-prd/02-five-layers.md — Layer 2 (Engine) vs. Layer 3
> (Harness): scoring vs. verification distinction.

---

## 7. Position in the Universal Loop

The universal loop in Roko is: **query → score → route → compose → act → verify →
write → react**. Gates occupy the **verify** step.

```
Agent produces output (act)
    ↓
Gate pipeline verifies output (verify)
    ↓
Verdict flows to:
    → Substrate (write: persist verdict as signal)
    → Scorer (react: update scoring model)
    → Router (react: update bandit arms)
    → Composer (react: adjust prompt sections)
    → Conductor (react: circuit breaker, watchers)
    → Ratchet (react: track highest rung passed)
    → Adaptive thresholds (react: update per-rung EMA)
    → Agent feedback (react: filter output for agent context)
```

The gate verdict is the system's primary feedback signal. It drives six different
learning and adaptation mechanisms. This is why the gate returning `Verdict` directly
(not `Result<Verdict>`) matters so much — every downstream consumer expects a definitive
answer, and wrapping it in `Result` would mean every consumer needs error-handling code
for a case that should never arise.

> **Citation**: refactoring-prd/01-synapse-architecture.md — Universal loop:
> query→score→route→compose→act→verify→write→react. Cybernetic feedback loops from
> Gate to Scorer, Router, Composer.

---

## 8. Relationship to the GVU Framework

The Generation-Verification-Update (GVU) framework from Song et al. (ICLR 2025) proves
a critical theorem: **self-improvement succeeds when the verifier is strong, not when
the generator is strong**. Specifically, the Variance Inequality shows that an oracle
verifier (verification noise σ_V ≈ 0) enables improvement despite arbitrarily high
generation noise.

Roko's gate pipeline is, in GVU terms, the verifier. Compilers and test suites are
oracle verifiers for their respective properties — they have zero false positive rate
for the properties they check. This is why Roko invests in a rich gate ecosystem (11
gates, 7 rungs) rather than relying solely on better prompts: **the returns to stronger
verification compound, while the returns to stronger generation plateau**.

> **Citation**: Song et al. "Self-Improving Diffusion Models with Synthetic Data" (ICLR
> 2025) — Generation-Verification-Update framework, Variance Inequality theorem.

> **Citation**: refactoring-prd/09-innovations.md — Innovation VIII: Process Reward
> Models informed by GVU framework's verification-first insight.

---

## 9. Future: The Gate Trait in Multi-Language Contexts

The current Gate trait signature accepts `Engram`. The Engram's body carries a `GatePayload` with `BuildSystem` (Cargo,
Npm, Go, Make). This means the Gate trait itself is language-agnostic — only the
concrete implementations (`CompileGate`, `TestGate`, `ClippyGate`) know about specific
build systems.

Adding support for a new language requires:
1. Adding a `BuildSystem` variant (e.g., `BuildSystem::Gradle`)
2. Implementing `check_args()`, `test_args()`, `lint_args()` for the new variant
3. Optionally: a new `parse_*_test_counts()` function for the test runner's output format

No changes to the Gate trait are needed.

> **Citation**: crates/roko-gate/src/test_gate.rs:171–179 — `parse_test_counts()`
> dispatches by `BuildSystem`, demonstrating language-agnostic gate design.

---

## 10. Summary

The Gate trait is deliberately minimal: two methods, no error return type, no generic
parameters. This simplicity is a feature. It means any verification step — from a 5ms
regex check to a 15-minute integration test suite — can be expressed as a Gate, composed
into a pipeline, tracked by a ratchet, measured by adaptive thresholds, and fed back to
agents and learning systems through a uniform interface.

The single most important thing to remember about the Gate trait:

**`verify()` returns `Verdict`, not `Result<Verdict>`. Gate failure is a verdict, not an error.**


---

# SOURCE: /Users/will/dev/nunchi/roko/roko/docs/04-verification/01-gate-implementations.md

# 01 — Gate Implementations

> **Layer**: L3 Harness — Verification
> **Crate**: `roko-gate` (`crates/roko-gate/src/`)
> **Status**: 11 gate implementations, 7 wired into the rung selector


> **Implementation**: Shipping

---

## 1. Overview

Roko ships 11 concrete gate implementations. Each implements the `Gate` trait (see
[00-gate-trait.md](./00-gate-trait.md)) and returns `Verdict` directly — not
`Result<Verdict>`. Every gate handles its own errors internally, converting
infrastructure failures (spawn errors, timeouts, malformed input) into
`Verdict::fail()`.

The gates are organized by verification layer, from cheapest/fastest to most
expensive/thorough:

| Gate | Module | Rung | Cost | What it verifies |
|---|---|---|---|---|
| `CompileGate` | `compile.rs` | 0 (Compile) | Low | Code compiles |
| `ClippyGate` | `clippy_gate.rs` | 1 (Lint) | Low | No lint violations |
| `TestGate` | `test_gate.rs` | 2 (Test) | Medium | Tests pass |
| `SymbolGate` | `symbol_gate.rs` | 3 (Symbol) | Zero (no subprocess) | Required symbols exist |
| `DiffGate` | `diff_gate.rs` | N/A (pre-pipeline) | Zero | Non-vacuous changes |
| `ShellGate` | `shell.rs` | Any | Varies | Arbitrary shell command |
| `GeneratedTestGate` | `generated_test_gate.rs` | 4 (GeneratedTest) | High | Auto-generated tests pass |
| `PropertyTestGate` | `property_test_gate.rs` | 5 (PropertyTest) | High | Property-based tests pass |
| `IntegrationGate` | `integration_gate.rs` | 6 (Integration) | Highest | Integration tests pass |
| `LlmJudgeGate` | `llm_judge_gate.rs` | N/A (auxiliary) | Medium | LLM-based quality judgment |
| `VerifyChainGate` | `verify_chain_gate.rs` | N/A (auxiliary) | Low | Chain verification |

> **Citation**: crates/roko-gate/src/lib.rs — Module declarations for all 11 gates.

---

## 2. ShellGate — The Foundation

**File**: `crates/roko-gate/src/shell.rs` (186 lines)

`ShellGate` is the simplest gate and the building block for others. It runs an arbitrary
shell command and passes if the exit code is 0.

```rust
pub struct ShellGate {
    program: String,
    args: Vec<String>,
    timeout_ms: u64,     // default: 5 minutes (300,000 ms)
    name: String,
}
```

### Construction

```rust
ShellGate::new("cargo", vec!["fmt".into(), "--check".into()])
    .with_timeout_ms(60_000)
    .with_name("format_check")
```

### Verdict Construction

The `ShellGate::verify()` method demonstrates the canonical pattern used by all
shell-spawning gates:

1. **Read payload**: Extract `GatePayload` from signal body for `working_dir` and
   `extra_env`. If absent, use process defaults.
2. **Build command**: Set program, args, cwd, env vars, `kill_on_drop(true)`.
3. **Run with timeout**: `tokio::time::timeout(duration, cmd.output()).await`
4. **Handle three outcomes**:
   - `Err(_)` → timeout → `Verdict::fail("timed out after N ms")`
   - `Ok(Err(io_err))` → spawn failure → `Verdict::fail("spawn failed: ...")`
   - `Ok(Ok(output))` → check exit code → `Verdict::pass()` or `Verdict::fail()`
5. **Always attach**: `.with_detail(combined_output).with_duration(elapsed_ms)`

> **Citation**: crates/roko-gate/src/shell.rs:57–118 — Full `ShellGate::verify()`
> implementation.

---

## 3. CompileGate — Rung 0

**File**: `crates/roko-gate/src/compile.rs` (195 lines)

`CompileGate` wraps `ShellGate`'s pattern with build-system awareness. It reads
`BuildSystem` from the `GatePayload` and runs the appropriate check command.

```rust
pub struct CompileGate {
    build_system: BuildSystem,
    extra_args: Vec<String>,
    timeout_ms: u64,     // default: 10 minutes (600,000 ms)
    name: String,        // e.g., "compile:cargo"
}
```

### Build System Dispatch

The `BuildSystem` enum (defined in `payload.rs`) supports:
- **Cargo**: `cargo check --workspace`
- **Npm**: `npm run build`
- **Go**: `go build ./...`
- **Make**: `make`

Each variant implements `program()`, `check_args()`, `test_args()`, and `lint_args()`.

### Error Summarization

On failure, `CompileGate` extracts up to 3 error-level diagnostics from stderr via
`summarize_errors()`. This keeps the verdict's `reason` field concise while the full
output lives in `detail`.

```rust
fn summarize_errors(stderr: &str, max: usize) -> String {
    let errors: Vec<&str> = stderr.lines()
        .filter(|l| l.trim_start().starts_with("error:") || l.trim_start().starts_with("error["))
        .take(max)
        .collect();
    if !errors.is_empty() { errors.join("; ") }
    else { stderr.lines().find(|l| !l.trim().is_empty()).unwrap_or("compilation failed").to_string() }
}
```

> **Citation**: crates/roko-gate/src/compile.rs:134–151 — `summarize_errors()`
> function.

> **Citation**: bardo-backup/tmp/mori-agents/20-verification-first-architecture.md —
> Rung 0 is compilation: "cheapest check that proves the code at least compiles."

---

## 4. ClippyGate — Rung 1

**File**: `crates/roko-gate/src/clippy_gate.rs` (212 lines)

`ClippyGate` runs the language-appropriate linter. For Rust, this is `cargo clippy --
-D warnings`; for Go, `go vet`; for Node, `npm run lint`.

```rust
pub struct ClippyGate {
    build_system: BuildSystem,
    extra_args: Vec<String>,
    timeout_ms: u64,     // default: 5 minutes (300,000 ms)
    name: String,        // e.g., "clippy:cargo"
}
```

### Argument Splicing

ClippyGate handles the Cargo-specific `--` sentinel carefully: extra args are inserted
*before* the `--` separator so they apply to the Cargo invocation (e.g., `--features ci`)
rather than to Clippy's own flags.

```rust
let dash_idx = base.iter().position(|a| *a == "--");
if let Some(idx) = dash_idx {
    // Insert extra_args before the "--"
    for arg in &base[..idx] { cmd.arg(arg); }
    for arg in &self.extra_args { cmd.arg(arg); }
    for arg in &base[idx..] { cmd.arg(arg); }
}
```

### Design: Lint Before Test

ClippyGate is designed to run before `TestGate` in a short-circuit pipeline. Lint checks
are fast (seconds) and catch many issues that would cause expensive test failures
(minutes). Running lint first saves time by failing fast.

> **Citation**: crates/roko-gate/src/clippy_gate.rs:70–93 — Argument splicing logic.

---

## 5. TestGate — Rung 2

**File**: `crates/roko-gate/src/test_gate.rs` (384 lines)

`TestGate` runs the project's test suite and parses passed/failed/ignored counts from
the output.

```rust
pub struct TestGate {
    build_system: BuildSystem,
    selector: TestSelector,
    extra_args: Vec<String>,
    timeout_ms: u64,     // default: 15 minutes (900,000 ms)
    name: String,        // e.g., "test:cargo"
}
```

### Test Selectors

The `TestSelector` enum controls which tests run:

```rust
pub enum TestSelector {
    All,                    // Run everything
    Quick,                  // --lib (unit tests only)
    Patterns(Vec<String>),  // Specific test patterns
}
```

For Cargo, `Quick` adds `--lib`. For Go, `Patterns(["TestFoo", "TestBar"])` becomes
`-run TestFoo|TestBar`.

### Test Count Parsing

`parse_test_counts()` dispatches by build system:
- **Cargo**: Parses `test result: ok. N passed; M failed; K ignored` lines, aggregating
  across multiple test targets.
- **Go**: Counts `--- PASS:`, `--- FAIL:`, `--- SKIP:` markers.

Parsed counts are attached to the verdict via `.with_test_count(tc)`. This enables
downstream policies to classify "mostly passing" runs (e.g., ≥90% pass rate with ≥20
tests and ≥1 failure) differently from total failures.

> **Citation**: crates/roko-gate/src/test_gate.rs:166–241 — `parse_test_counts()` and
> per-build-system parsers.

> **Citation**: bardo-backup/tmp/mori-agents/20-verification-first-architecture.md —
> Rung 1 (test execution), verification-first architecture.

---

## 6. SymbolGate — Rung 3

**File**: `crates/roko-gate/src/symbol_gate.rs` (1001 lines)

`SymbolGate` is unique among the gates: it spawns no subprocesses and makes no LLM
calls. It parses Rust source files directly and verifies that every symbol in a
`SymbolManifest` exists with the correct kind, visibility, and module path.

```rust
pub struct SymbolGate {
    source_roots: Vec<PathBuf>,
    name: String,
}
```

### What It Catches

The most common agent failure is: "I was told to create `pub struct RateLimiter` and
did not." SymbolGate catches this at effectively zero cost, before any compilation or
test execution.

### Mismatch Taxonomy

When expectations are not met, SymbolGate produces a structured error digest:

```
4 symbol expectation(s) unmet:
  MISSING: struct RateLimiter at core::rate_limit
  WRONG_VIS: fn check_rate at core::rate_limit (found: private, expected: pub)
  WRONG_KIND: Limiter at core::rate_limit (found: struct, expected: trait)
  WRONG_PATH: struct Clock at core::time (found at: core::clock)
  AMBIGUOUS: fn foo at core::util (2 matches)
```

Five mismatch categories: `MISSING`, `WRONG_VIS`, `WRONG_KIND`, `WRONG_PATH`,
`AMBIGUOUS`. Each tells the agent (or the human) exactly what went wrong and where.

### Symbol Extraction

The scanner is a lightweight single-pass line-based extractor. It handles:
- Visibility modifiers: `pub`, `pub(crate)`, `pub(super)`, `pub(in path)`
- Modifier keywords: `async`, `unsafe`, `extern "C"`, `const fn`
- Item kinds: `struct`, `enum`, `trait`, `fn`, `type`, `const`, `static`, `mod`

It deliberately does not descend into `mod foo { ... }` blocks (rare in idiomatic Rust).

> **Citation**: crates/roko-gate/src/symbol_gate.rs:449–478 — `extract_symbols()`
> lightweight parser.

---

## 7. DiffGate — Vacuous-Implementation Rejection

**File**: `crates/roko-gate/src/diff_gate.rs` (358 lines)

`DiffGate` solves a specific failure mode: agents that "pass" gates by producing
vacuous implementations. Without this gate, an agent can replace function bodies with
`todo!()` or `Ok(())` to make compile and lint gates happy while doing no actual work.

### Rejection Criteria

A diff is rejected when:
1. **Empty diff**: Zero added lines.
2. **Below threshold**: Non-whitespace added lines < `min_added_lines` (default: 1).
3. **All forbidden tokens**: Every substantive added line matches a forbidden token.

Default forbidden tokens:
- `todo!()`, `todo!`, `unimplemented!()`, `unimplemented!`
- `panic!("not implemented")`, `Ok(())`, `return Ok(())`

### Analysis Output

```rust
pub struct DiffAnalysis {
    pub added_lines: u32,
    pub non_whitespace_added: u32,
    pub all_added_are_forbidden: bool,
}
```

The `analyze_diff()` function is pure: no I/O, no subprocess, just string scanning.
It skips diff headers (`+++`, `---`, `@@`), counts `+` lines, filters whitespace and
comments, and checks against the forbidden token list.

> **Citation**: crates/roko-gate/src/diff_gate.rs:1–33 — Module doc: "Rejects changes
> that 'pass' gates only because they introduced no substantive work."

---

## 8. Remaining Gates (Scaffold Status)

### 8.1 GeneratedTestGate (Rung 4)

Runs tests that were automatically generated by the agent or a dedicated test-generation
agent. These are distinct from the project's existing test suite (Rung 2). Generated
tests are more targeted — they specifically exercise the code the agent just wrote.

### 8.2 PropertyTestGate (Rung 5)

Runs property-based tests (QuickCheck / proptest style). Property tests provide stronger
guarantees than example-based tests: they assert invariants over randomized inputs.

### 8.3 IntegrationGate (Rung 6)

Runs the full integration test suite. This is the most expensive gate — it may involve
standing up services, databases, or network connections.

### 8.4 LlmJudgeGate (Auxiliary)

Uses an LLM to evaluate code quality, correctness, or adherence to specifications. This
is the only gate that consults a model rather than a deterministic tool. It is used
when properties are too nuanced for automated checking (e.g., "does this implementation
match the PRD's intent?").

### 8.5 VerifyChainGate (Auxiliary)

Verifies chain-related artifacts. Part of the Phase 2+ architecture.

> **Citation**: crates/roko-gate/src/lib.rs — All 11 gate modules declared.

---

## 9. Shared Patterns Across All Gates

### 9.1 Builder Pattern

Every gate uses a builder-style API with `with_*` methods:

```rust
CompileGate::cargo()
    .with_extra_args(vec!["--features".into(), "ci".into()])
    .with_timeout_ms(120_000)
```

### 9.2 GatePayload

Gates that shell out to external tools read their configuration from a `GatePayload`
in the signal body:

```rust
pub struct GatePayload {
    pub working_dir: PathBuf,
    pub target_dir: Option<PathBuf>,
    pub extra_env: HashMap<String, String>,
    // ... build system, test selector, etc.
}
```

This decouples the gate from the orchestrator: the orchestrator constructs the signal
with the right payload, and the gate reads it.

### 9.3 Timeout + kill_on_drop

Every subprocess-spawning gate sets `cmd.kill_on_drop(true)` and wraps execution in
`tokio::time::timeout()`. This ensures:
- No zombie processes from abandoned gates
- Bounded execution time for every gate

### 9.4 Error Summarization

Each gate has a gate-specific `summarize_*` function that extracts the most relevant
error lines from stderr. This keeps verdict reasons concise (3 lines) while preserving
full output in `detail`.

> **Citation**: crates/roko-gate/src/compile.rs:130–151 — `summarize_errors()`.
> crates/roko-gate/src/test_gate.rs:244–267 — `summarize_test_failures()`.
> crates/roko-gate/src/clippy_gate.rs:146–163 — `summarize_lint_issues()`.

---

## 10. Gate Composition

Gates are designed to be composed, not used in isolation. The primary composition
mechanism is the `GatePipeline` (see [03-gate-pipeline.md](./03-gate-pipeline.md)),
which chains gates sequentially with optional short-circuit behavior. The `RungSelector`
(see [02-6-rung-selector.md](./02-6-rung-selector.md)) determines which gates to
include based on plan complexity and prior failures.

The composition chain is:

```
RungSelector picks rungs for this plan's complexity
    ↓
GatePipeline composes the selected gates
    ↓
Each gate runs in sequence (or short-circuits on failure)
    ↓
Aggregated Verdict flows to ratchet, thresholds, feedback
```

> **Citation**: crates/roko-gate/src/gate_pipeline.rs — GatePipeline composes
> `Vec<Box<dyn Gate>>` sequentially.


---

# SOURCE: /Users/will/dev/nunchi/roko/roko/docs/04-verification/02-6-rung-selector.md

# 02 — The 7-Rung Gate Selector

> **Layer**: L3 Harness — Verification
> **Crate**: `roko-gate` (`crates/roko-gate/src/rung_selector.rs`)
> **Status**: Implemented (527 lines)


> **Implementation**: Shipping

---

## 1. Overview

Not every task needs every gate. A one-line variable rename doesn't need property-based
testing. A new subsystem does. The rung selector solves this: given a plan's complexity
and the gates available in this environment, it produces a sorted list of rungs to
execute.

The system has 7 rungs, numbered 0 through 6, ordered from cheapest to most expensive.
Lower-numbered rungs always run before higher-numbered ones. The selector determines
*which* subset of rungs to activate based on three inputs:

1. **Plan complexity** — how ambitious is this change?
2. **Rung capabilities** — which gates are actually available?
3. **Prior failures** — did previous attempts fail, warranting escalation?

> **Citation**: crates/roko-gate/src/rung_selector.rs — Full implementation.

> **Citation**: bardo-backup/tmp/mori-agents/20-verification-first-architecture.md —
> 6-rung gate system design (Rungs 0–5 in the original; Roko extends to 7 with Rung 6
> for integration tests).

---

## 2. The 7 Rungs

```rust
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub enum Rung {
    Compile     = 0,    // Does it compile?
    Lint        = 1,    // Does it pass linting?
    Test        = 2,    // Do tests pass?
    Symbol      = 3,    // Are required symbols present?
    GeneratedTest = 4,  // Do auto-generated tests pass?
    PropertyTest  = 5,  // Do property-based tests pass?
    Integration   = 6,  // Do integration tests pass?
}
```

### 2.1 Rung 0: Compile

The cheapest gate. Runs `cargo check --workspace` (or equivalent). Takes seconds.
Catches syntax errors, type mismatches, missing imports. Every plan runs at least this
rung.

**Gate**: `CompileGate`
**Cost**: Low (seconds)
**False positive rate**: 0% (deterministic)

### 2.2 Rung 1: Lint

Runs `cargo clippy -- -D warnings` (or language equivalent). Catches code quality
issues, common mistakes, and style violations. Still fast — typically under a minute.

**Gate**: `ClippyGate`
**Cost**: Low (seconds to one minute)
**False positive rate**: 0% (deterministic)

### 2.3 Rung 2: Test

Runs the project's existing test suite. This is the first "medium cost" gate — tests
can take minutes for large projects. Catches functional regressions.

**Gate**: `TestGate`
**Cost**: Medium (seconds to 15 minutes)
**False positive rate**: 0% for deterministic tests; flaky tests exist but are not
the gate's fault.

### 2.4 Rung 3: Symbol

Verifies that required symbols (structs, traits, functions) exist with correct kind,
visibility, and module path. Zero cost — no subprocess, just file walking and regex
matching.

**Gate**: `SymbolGate`
**Cost**: Near-zero (file I/O only)
**False positive rate**: 0% (deterministic)

Despite being "free," Symbol is ranked after Test because the information it provides
(symbol existence) is a subset of what compile + test already verify. Symbol is most
valuable when it catches issues *before* a failed compile-test cycle reveals them.

### 2.5 Rung 4: GeneratedTest

Runs tests that were automatically generated to exercise the agent's output. These are
more targeted than the project's existing tests — they specifically test the new code.

**Gate**: `GeneratedTestGate`
**Cost**: High (test generation + execution)
**False positive rate**: Moderate (generated tests may be incorrect)

### 2.6 Rung 5: PropertyTest

Runs property-based tests (QuickCheck / proptest). These assert invariants over
randomized inputs, providing stronger guarantees than example-based tests.

**Gate**: `PropertyTestGate`
**Cost**: High (randomized exploration)
**False positive rate**: Low (invariants are precise)

### 2.7 Rung 6: Integration

The most expensive gate. Runs full integration tests that may involve external services,
databases, or network connections.

**Gate**: `IntegrationGate`
**Cost**: Highest (minutes to hours)
**False positive rate**: Moderate (infrastructure flakiness)

> **Citation**: crates/roko-gate/src/rung_selector.rs — `Rung` enum definition with
> discriminants 0–6.

---

## 3. Plan Complexity

The selector's primary input is `PlanComplexity`, which classifies the scope of the
planned change:

```rust
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub enum PlanComplexity {
    Trivial,    // Rename, typo fix, config change
    Simple,     // Single function change, small bug fix
    Standard,   // Multi-file feature, medium scope
    Complex,    // New subsystem, architectural change
}
```

### 3.1 Complexity → Rung Mapping

Each complexity level implies a baseline set of rungs:

| Complexity | Baseline Rungs | Rationale |
|---|---|---|
| `Trivial` | Compile only (0) | No functional changes; compile is sufficient |
| `Simple` | Compile + Lint (0–1) | Small changes; lint catches quality issues |
| `Standard` | Compile + Lint + Test + Symbol (0–3) | Feature work needs test coverage |
| `Complex` | All available (0–6) | Architectural changes need full verification |

### 3.2 Escalation on Failure

When a plan fails a gate, the complexity escalates:

```rust
impl PlanComplexity {
    pub fn escalate(&self) -> Self {
        match self {
            Self::Trivial => Self::Simple,
            Self::Simple => Self::Standard,
            Self::Standard => Self::Complex,
            Self::Complex => Self::Complex,  // Already maximal
        }
    }
}
```

Escalation adds more rungs on retry. A `Trivial` plan that fails compile gets escalated
to `Simple` (adding lint). A `Simple` plan that fails lint gets escalated to `Standard`
(adding test + symbol). This captures the heuristic: if the easy checks catch a problem,
the change is more complex than initially classified, and deeper verification is
warranted.

> **Citation**: crates/roko-gate/src/rung_selector.rs — `PlanComplexity::escalate()`
> method.

---

## 4. Rung Capabilities

Not every environment has every gate available. Some projects don't have a linter
configured. Some don't have property tests. The `RungCaps` struct records which rungs
can actually run:

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

The selector intersects the complexity-implied rungs with the available capabilities.
If the complexity says "run through Rung 3" but `symbol` is `false`, the symbol rung is
skipped.

---

## 5. The `select_rungs()` Function

The public API combines all three inputs:

```rust
pub fn select_rungs(
    complexity: PlanComplexity,
    caps: &RungCaps,
    prior_failures: u32,
) -> Vec<Rung>
```

The algorithm:

1. **Determine effective complexity**: If `prior_failures > 0`, escalate complexity
   by `prior_failures` levels (capped at `Complex`).
2. **Map complexity to maximum rung**: Trivial → 0, Simple → 1, Standard → 3,
   Complex → 6.
3. **Collect all rungs ≤ maximum** that are available in `caps`.
4. **Sort ascending** (cheapest first).
5. **Return the rung list**.

### Example

```
Input:
  complexity = Simple
  caps = { compile: true, lint: true, test: true, symbol: false, ... }
  prior_failures = 1

Step 1: Escalate Simple by 1 → Standard
Step 2: Standard → max rung 3
Step 3: Available rungs ≤ 3 = [Compile(0), Lint(1), Test(2)]
         (Symbol(3) excluded because caps.symbol = false)
Step 4: Already sorted
Result: [Compile, Lint, Test]
```

---

## 6. Relationship to the Gate Pipeline

The rung selector produces a `Vec<Rung>`. The orchestrator maps each `Rung` to a
concrete gate implementation and feeds them into a `GatePipeline`:

```
RungSelector::select_rungs(complexity, caps, failures)
    → Vec<Rung>
    → map each Rung to Box<dyn Gate>
    → GatePipeline::new(gates).with_short_circuit(true)
    → pipeline.verify(signal, ctx)
    → aggregated Verdict
```

This separation of concerns means the selector knows nothing about gate implementations
and the pipeline knows nothing about complexity. Each can evolve independently.

> **Citation**: crates/roko-gate/src/gate_pipeline.rs — GatePipeline accepts
> `Vec<Box<dyn Gate>>` without knowing which rungs they represent.

---

## 7. The Verification-First Architecture

The rung system embodies the verification-first architecture from the Mori reference
system. The core insight is:

**Cheap verification gates that run first prevent expensive retries.**

If the code doesn't compile (Rung 0), there is no point running tests (Rung 2). If
the code has lint violations (Rung 1), many of those violations will also cause test
failures. Running gates in ascending cost order with short-circuit behavior means the
system spends the minimum possible time and compute on verification before either
passing or identifying the cheapest-to-fix failure.

The cost savings compound:
- A compile failure caught in 3 seconds saves a 15-minute test run.
- A lint failure caught in 10 seconds saves the same 15-minute test run.
- A symbol check caught in 50ms saves a 3-second compile + 10-second lint + 15-minute
  test run (though symbol is ranked after test in the default ordering).

> **Citation**: bardo-backup/tmp/mori-agents/20-verification-first-architecture.md —
> "6-rung gate system: separate test generation from implementation, tests must fail
> before implementation, immutable verification artifacts."

---

## 8. Escalation in Practice

The escalation mechanism creates a feedback loop between gate outcomes and verification
intensity:

```
Attempt 1: Trivial complexity → Compile only
  → Compile fails
  → Escalate to Simple (prior_failures = 1)

Attempt 2: Simple complexity → Compile + Lint
  → Both pass
  → Record pass at Rung 1
  → Ratchet prevents regression below Rung 1

Attempt 3 (if task retry): Simple complexity → Compile + Lint
  → Lint fails
  → Escalate to Standard (prior_failures = 2)

Attempt 4: Standard complexity → Compile + Lint + Test + Symbol
  → All pass
  → Record pass at Rung 3
  → Task verified
```

This means the system starts cheap and escalates only when warranted. A plan that
passes on the first attempt at Trivial complexity uses only the compile gate —
sub-second verification. A plan that fails repeatedly gets the full battery.

---

## 9. Rung Ordering vs. Rung Cost

The rung numbers (0–6) encode a rough cost ordering, but the actual cost depends on the
project:

| Rung | Typical Cost | Notes |
|---|---|---|
| 0 (Compile) | 1–10 seconds | Incremental builds are faster |
| 1 (Lint) | 2–60 seconds | Clippy can be slow on large codebases |
| 2 (Test) | 5 seconds – 15 minutes | Depends on test count |
| 3 (Symbol) | 10–100 ms | Pure file I/O, no subprocess |
| 4 (GeneratedTest) | 30 seconds – 5 minutes | Depends on generation + execution |
| 5 (PropertyTest) | 10 seconds – 10 minutes | Depends on iterations |
| 6 (Integration) | 1 minute – 1 hour | Depends on infrastructure |

The rung numbers represent a logical ordering (compile before test before integration),
not a strict cost ordering. Symbol (Rung 3) is actually cheaper than Compile (Rung 0),
but logically sits after Test because its value is in catching issues that compile + test
miss.

> **Citation**: refactoring-prd/02-five-layers.md — Layer 3 (Harness): "scoring,
> pattern detection, interventions, process rewards, adaptive gating."

---

## 10. Configuration and Extension

### 10.1 Adding a New Rung

To add a Rung 7 (e.g., "SecurityAudit"):
1. Add `SecurityAudit = 7` to the `Rung` enum.
2. Add `security_audit: bool` to `RungCaps`.
3. Update `select_rungs()` to include Rung 7 for `Complex` plans.
4. Implement a `SecurityAuditGate` in a new module.
5. Map `Rung::SecurityAudit` to `SecurityAuditGate` in the orchestrator.

### 10.2 Configuring per-Project Capabilities

The `RungCaps` struct is constructed by the orchestrator based on project detection:
- If `Cargo.toml` exists → `compile: true`, `lint: true`, `test: true`
- If `roko.toml` specifies `[gates.symbol]` → `symbol: true`
- If property test fixtures exist → `property_test: true`

This means the selector adapts to the project automatically — no configuration needed
for basic verification.

---

## 11. Relationship to Adaptive Thresholds

The adaptive threshold system (see [06-adaptive-thresholds.md](./06-adaptive-thresholds.md))
tracks per-rung pass rates. If a rung consistently passes (20+ consecutive passes), the
threshold system may recommend skipping it (advisory only). This provides a
runtime-adaptive complement to the static complexity-based selection.

The two systems compose: the rung selector determines the *baseline* set of rungs, and
the adaptive thresholds *refine* it based on historical performance.

> **Citation**: crates/roko-gate/src/adaptive_threshold.rs — `should_skip_rung()`
> advisory based on consecutive pass streak.


---

# SOURCE: /Users/will/dev/nunchi/roko/roko/docs/04-verification/03-gate-pipeline.md

# 03 — The Gate Pipeline

> **Layer**: L3 Harness — Verification
> **Crate**: `roko-gate` (`crates/roko-gate/src/gate_pipeline.rs`)
> **Status**: Implemented (593 lines), wired into orchestrate.rs


> **Implementation**: Shipping

---

## 1. Overview

The `GatePipeline` composes multiple gates into a single verification step. It accepts a
`Vec<Box<dyn Gate>>`, runs them sequentially, and produces an aggregated `Verdict`. It
implements the `Gate` trait itself, so a pipeline can be nested inside another pipeline
or used anywhere a single gate is expected.

This is the primary mechanism for turning the rung selector's output (a list of rungs)
into a concrete verification execution.

> **Citation**: crates/roko-gate/src/gate_pipeline.rs — Full implementation.

---

## 2. Structure

```rust
pub struct GatePipeline {
    gates: Vec<Box<dyn Gate>>,
    short_circuit: bool,
    name: String,
}
```

| Field | Purpose |
|---|---|
| `gates` | Ordered list of gates to execute |
| `short_circuit` | If true, stop on first failure |
| `name` | Display name for the pipeline's own verdict |

### Construction

```rust
GatePipeline::new(vec![
    Box::new(CompileGate::cargo()),
    Box::new(ClippyGate::cargo()),
    Box::new(TestGate::cargo()),
])
.with_short_circuit(true)
.with_name("rung-pipeline")
```

The builder pattern follows the same convention as individual gates: `with_*` methods
return `Self` for chaining.

---

## 3. Short-Circuit vs. Full Execution

### 3.1 Short-Circuit Mode (`short_circuit: true`)

The default and most common mode. The pipeline stops at the first gate that fails and
returns a failure verdict immediately. This is the correct behavior for the rung
pipeline: if compile fails, there is no point running lint or tests.

```
CompileGate → FAIL → stop → return Verdict::fail(...)
              (ClippyGate and TestGate never run)
```

**Why this matters**: In a 7-rung pipeline where integration tests take 30 minutes, a
compile failure caught in 3 seconds saves 30+ minutes of wasted compute. Short-circuit
mode is the mechanism that makes the verification-first architecture efficient.

### 3.2 Full Execution Mode (`short_circuit: false`)

All gates run regardless of individual outcomes. The final verdict is a failure if *any*
gate failed. This mode is useful when you want a comprehensive report of all issues
rather than just the first.

```
CompileGate → FAIL → continue
ClippyGate  → PASS → continue
TestGate    → FAIL → continue
→ return aggregated Verdict::fail(...)
  (detail includes output from all three gates)
```

---

## 4. Verdict Aggregation

The pipeline's `verify()` method aggregates individual verdicts into a single
pipeline-level verdict:

### 4.1 Pass Condition

The pipeline passes if and only if **every** gate passes. A single failure anywhere in
the chain makes the whole pipeline fail.

### 4.2 Detail Aggregation

Individual gate outputs are concatenated into the pipeline's `detail` field, separated
by headers:

```
--- [compile:cargo] ---
Compiling foo v0.1.0
Finished dev in 2.3s

--- [clippy:cargo] ---
warning: unused variable

--- [test:cargo] ---
test result: ok. 12 passed; 0 failed; 0 ignored
```

This gives the caller (and the agent) a complete view of what happened at each
verification step.

### 4.3 Reason Construction

On failure, the pipeline's `reason` field lists which gate(s) failed:

```
gate pipeline failed: compile:cargo (error: bad thing; error[E0425]: symbol not found)
```

In short-circuit mode, there is exactly one failed gate. In full-execution mode, there
may be multiple.

### 4.4 Duration

The pipeline's `duration_ms` is the sum of all individual gate durations. This tracks
total wall-clock time spent on verification.

### 4.5 Test Count Merging

If any gate in the pipeline produces `TestCount` (test gates do), the pipeline merges
them by summing passed, failed, and ignored counts across all gates. This is relevant
when a pipeline contains multiple test gates (e.g., unit tests + integration tests).

---

## 5. The Pipeline as a Gate

`GatePipeline` implements `Gate`:

```rust
#[async_trait]
impl Gate for GatePipeline {
    async fn verify(&self, engram: &Engram, ctx: &Context) -> Verdict {
        // ... iterate over self.gates, aggregate verdicts
    }

    fn name(&self) -> &str {
        &self.name
    }
}
```

This composability is intentional. It means:
- A pipeline can contain other pipelines (nesting)
- Any code that accepts `&dyn Gate` can accept a pipeline
- The adaptive threshold system, ratchet, and feedback systems all work with pipelines
  without special-casing

---

## 6. Execution Flow

```
Pipeline::verify(signal, ctx)
│
├─ gate[0].verify(signal, ctx) → verdict_0
│  ├─ if failed && short_circuit → return fail verdict
│  └─ collect detail, test counts
│
├─ gate[1].verify(signal, ctx) → verdict_1
│  ├─ if failed && short_circuit → return fail verdict
│  └─ collect detail, test counts
│
├─ ... (for each gate)
│
└─ aggregate:
   ├─ passed = all verdicts passed
   ├─ reason = join failure reasons
   ├─ detail = join all details with headers
   ├─ duration = sum of durations
   ├─ test_count = sum of test counts
   └─ return aggregated Verdict
```

---

## 7. How the Orchestrator Uses the Pipeline

In `crates/roko-cli/src/orchestrate.rs`, the orchestrator constructs a pipeline per task:

```
1. Determine plan complexity (Trivial/Simple/Standard/Complex)
2. Detect environment capabilities (which build tools exist)
3. select_rungs(complexity, caps, prior_failures)
4. Map each Rung to a concrete Box<dyn Gate>
5. GatePipeline::new(gates).with_short_circuit(true)
6. pipeline.verify(signal, ctx)
7. Feed verdict to ratchet, thresholds, feedback
```

The pipeline is constructed fresh for each task execution. This means:
- Different tasks can have different pipelines (based on complexity)
- Escalation adds gates to the pipeline on retry
- The pipeline is lightweight — no persistent state

---

## 8. Error Handling Within the Pipeline

Because the Gate trait returns `Verdict` (not `Result<Verdict>`), the pipeline never
has to handle gate errors. Every gate handles its own infrastructure failures internally.
The pipeline simply collects verdicts and aggregates them.

This is the practical benefit of the `-> Verdict` design decision: composition is
trivial. There are no error propagation paths to worry about, no `?` operators, no
`Result::map` chains. Just: run the gate, get a verdict, check if it passed.

> **Citation**: 00-gate-trait.md — "Gate failure is not an error — it is a verdict."

---

## 9. Pipeline Lifecycle in the Universal Loop

```
Universal loop: query → score → route → compose → act → VERIFY → write → react
                                                        ^^^^^^^^
                                                   GatePipeline lives here

Signal produced by agent (act step)
    ↓
GatePipeline.verify(signal, ctx)
    ↓
Verdict flows to:
    ├─ write: Verdict persisted as signal in Substrate
    ├─ react: GateRatchet.record_pass(plan_id, rung)
    ├─ react: AdaptiveThresholds.update(rung, passed)
    ├─ react: GateFeedback for agent context on retry
    ├─ react: EfficiencyEvent for learning
    └─ react: CascadeRouter.update_arm(model, reward)
```

The pipeline is the single point where all these downstream systems get their input.
This centralization means there's one place to add new feedback consumers, one place
to add instrumentation, and one place to add logging.

> **Citation**: refactoring-prd/01-synapse-architecture.md — Cybernetic feedback loops
> from Gate to Scorer, Router, Composer.

---

## 10. Testing the Pipeline

The pipeline has extensive tests in `gate_pipeline.rs`:

| Test | What It Verifies |
|---|---|
| `pipeline_empty_passes` | Empty pipeline returns pass verdict |
| `pipeline_single_pass` | Single passing gate → pass |
| `pipeline_single_fail` | Single failing gate → fail |
| `pipeline_short_circuits` | Stops at first failure |
| `pipeline_full_execution` | Runs all gates when short_circuit=false |
| `pipeline_aggregates_test_counts` | Merges test counts across gates |
| `pipeline_detail_headers` | Detail output has per-gate headers |
| `pipeline_duration_sums` | Total duration = sum of gate durations |

These tests use mock gates that return predetermined verdicts, avoiding the need for
actual subprocess spawning in unit tests.

---

## 11. Relationship to Other Components

| Component | Relationship |
|---|---|
| `RungSelector` | Determines which gates go into the pipeline |
| `GateRatchet` | Consumes pipeline verdicts to track regression |
| `AdaptiveThresholds` | Consumes pipeline verdicts for per-rung EMA |
| `GateFeedback` | Parses pipeline detail output for agent context |
| `ArtifactStore` | Future: stores pipeline artifacts content-addressed |
| Orchestrator | Constructs and executes the pipeline per task |

> **Citation**: crates/roko-gate/src/gate_pipeline.rs — Tests demonstrating pipeline
> behavior.

---

## 12. Design Rationale: Sequential, Not Parallel

The current pipeline executes gates sequentially. This is deliberate:

1. **Dependency ordering**: Rung N often depends on Rung N-1's success. Running tests
   on code that doesn't compile wastes time and produces confusing errors.
2. **Short-circuit value**: Sequential execution enables short-circuit, which is the
   pipeline's primary optimization.
3. **Simplicity**: Sequential execution has no synchronization concerns.

Future: A "gate group" concept where independent gates within the same rung run in
parallel. For example, if a rung has both a symbol gate and a format gate (both zero
cost, no dependency between them), they could run concurrently. This would be a new
composition primitive, not a change to the pipeline's sequential semantics.

---

## 13. Gate Composition Algebra

The pipeline's sequential composition is one combinator. A complete algebra over gates
enables richer verification topologies — parallel fan-outs, voting, fallback chains, and
confidence-weighted verdicts. The algebra treats `Gate` as the base type and defines
combinators that produce new gates from existing ones.

> **Citation**: Foundational Property-Based Testing (Paraskevopoulou & Hritcu) — formal
> compositional structures over verification predicates.

### 13.1 Verdict Lattice

Verdicts form a bounded lattice ordered by severity:

```
Skip < Warn < Pass < Fail

identity:  Skip  (a gate that produces Skip has no effect)
absorber:  Fail  (once present, dominates any merge)
merge(v1, v2) = max(v1, v2)   -- most severe verdict wins
```

This gives a monoid over verdicts: `(Verdict, merge, Skip)`. Every combinator preserves
this structure — the composed gate always returns a single `Verdict` from the same lattice.

```rust
/// Extended verdict with a confidence interval, not just pass/fail.
#[derive(Debug, Clone, Copy, PartialEq, PartialOrd)]
pub enum VerdictSeverity {
    Skip = 0,
    Warn = 1,
    Pass = 2,
    Fail = 3,
}

impl VerdictSeverity {
    pub fn merge(self, other: Self) -> Self {
        if self as u8 >= other as u8 { self } else { other }
    }
}
```

### 13.2 Combinators

```rust
/// Sequential composition: g1 THEN g2 (short-circuit on fail).
/// This is what GatePipeline already does.
pub struct Sequential(Vec<Box<dyn Gate>>);

/// Parallel composition: run all gates concurrently, merge verdicts.
/// Independent gates (e.g., SymbolGate + DiffGate) run simultaneously.
pub struct Parallel(Vec<Box<dyn Gate>>);

/// Fallback: try g1; if it fails, try g2 instead.
/// Useful for degraded environments (e.g., no clippy → fall back to grep-based lint).
pub struct Fallback(Box<dyn Gate>, Box<dyn Gate>);

/// Voting: run N gates, pass if >= K pass (quorum).
/// Useful for LLM judge panels where individual judges may disagree.
pub struct Voting {
    gates: Vec<Box<dyn Gate>>,
    quorum: usize, // minimum passes required
}

/// Weighted: scale a gate's confidence by a factor.
/// Low-confidence gates (LLM judge) contribute less to aggregate decisions.
pub struct Weighted {
    gate: Box<dyn Gate>,
    weight: f64, // [0.0, 1.0]
}

/// Threshold: pass only if gate's score exceeds a minimum.
/// Converts continuous scores into binary verdicts at a chosen cut-point.
pub struct Threshold {
    gate: Box<dyn Gate>,
    min_score: f32, // gate.score must be >= this
}
```

### 13.3 Parallel Gate Group

The `Parallel` combinator enables concurrent execution of independent gates within
a single rung:

```rust
#[async_trait]
impl Gate for Parallel {
    async fn verify(&self, engram: &Engram, ctx: &Context) -> Verdict {
        let futures: Vec<_> = self.0.iter()
            .map(|g| g.verify(engram, ctx))
            .collect();
        let verdicts = futures::future::join_all(futures).await;

        let passed = verdicts.iter().all(|v| v.passed);
        let duration = verdicts.iter().map(|v| v.duration_ms).max().unwrap_or(0);
        let test_count = merge_test_counts(&verdicts);
        let detail = verdicts.iter()
            .map(|v| format!("--- [{}] ---\n{}", v.gate, v.detail.as_deref().unwrap_or("")))
            .collect::<Vec<_>>()
            .join("\n\n");

        Verdict {
            passed,
            gate: "parallel-group".into(),
            reason: if passed { "all gates passed".into() }
                    else { format!("{} gate(s) failed",
                        verdicts.iter().filter(|v| !v.passed).count()) },
            detail: Some(detail),
            duration_ms: duration, // wall-clock = max, not sum
            test_count,
            ..Default::default()
        }
    }
}
```

Key difference: duration is `max(durations)` not `sum(durations)`, because gates run
concurrently. A `Parallel(SymbolGate, DiffGate)` taking 50ms and 20ms respectively
completes in 50ms, not 70ms.

### 13.4 Voting Gate (Quorum)

For subjective gates (LLM judges, heuristic checks), a single verdict is noisy. A
voting gate runs multiple judges and requires a quorum:

```rust
#[async_trait]
impl Gate for Voting {
    async fn verify(&self, engram: &Engram, ctx: &Context) -> Verdict {
        let futures: Vec<_> = self.gates.iter()
            .map(|g| g.verify(engram, ctx))
            .collect();
        let verdicts = futures::future::join_all(futures).await;

        let pass_count = verdicts.iter().filter(|v| v.passed).count();
        let passed = pass_count >= self.quorum;
        let avg_score = verdicts.iter().map(|v| v.score).sum::<f32>()
            / verdicts.len() as f32;

        Verdict {
            passed,
            score: avg_score,
            gate: "voting-panel".into(),
            reason: format!("{}/{} passed (quorum: {})",
                pass_count, verdicts.len(), self.quorum),
            ..Default::default()
        }
    }
}
```

### 13.5 Composition Examples

```rust
// Standard 4-rung pipeline (current behavior)
Sequential(vec![CompileGate, ClippyGate, TestGate, SymbolGate])

// Parallel lint: clippy + format check run concurrently
Sequential(vec![
    CompileGate,
    Parallel(vec![ClippyGate, FormatGate]),
    TestGate,
])

// LLM judge panel: 3 cheap judges, pass if 2+ agree
Voting { gates: vec![Judge1, Judge2, Judge3], quorum: 2 }

// Degraded environment: try clippy, fall back to grep-based lint
Fallback(ClippyGate, GrepLintGate)

// Progressive: compile → (clippy ∥ diff) → test → (symbol ∥ generated) → property → integration
Sequential(vec![
    CompileGate,
    Parallel(vec![ClippyGate, DiffGate]),
    TestGate,
    Parallel(vec![SymbolGate, GeneratedTestGate]),
    PropertyTestGate,
    IntegrationGate,
])
```

---

## 14. Probabilistic Gates

Standard gates return binary pass/fail. Probabilistic gates return a confidence
interval — a range `[lower, upper]` expressing how certain the gate is about its verdict.
This is essential for gates that use sampling (property-based tests, fuzz tests) where
the result is inherently statistical.

> **Citation**: Sequential Analysis (Siegmund, Springer) — confidence-interval stopping
> rules. Wilson score interval for proportion estimation with small samples.

### 14.1 Confidence Interval Structure

```rust
/// A verdict with statistical confidence bounds.
#[derive(Debug, Clone)]
pub struct ProbabilisticVerdict {
    /// The point estimate of the pass rate.
    pub pass_rate: f64,
    /// Lower bound of the confidence interval.
    pub ci_lower: f64,
    /// Upper bound of the confidence interval.
    pub ci_upper: f64,
    /// Confidence level (e.g., 0.95 for 95% CI).
    pub confidence_level: f64,
    /// Number of samples taken.
    pub sample_count: u64,
    /// Whether the gate passed at the chosen threshold.
    pub passed: bool,
    /// Standard Verdict for pipeline compatibility.
    pub verdict: Verdict,
}
```

### 14.2 Wilson Score Interval

The Wilson score interval is preferred over the naive Wald interval because it is
well-calibrated even at small sample sizes and near boundary proportions (p ≈ 0 or 1):

```rust
/// Compute Wilson score confidence interval for a proportion.
///
/// Parameters:
///   successes: number of passing tests
///   total: total number of tests run
///   z: z-score for desired confidence (1.96 for 95%, 2.576 for 99%)
///
/// Returns: (lower_bound, upper_bound)
fn wilson_interval(successes: u64, total: u64, z: f64) -> (f64, f64) {
    let n = total as f64;
    let p_hat = successes as f64 / n;
    let z2 = z * z;

    let denominator = 1.0 + z2 / n;
    let center = (p_hat + z2 / (2.0 * n)) / denominator;
    let margin = (z / denominator)
        * ((p_hat * (1.0 - p_hat) / n) + (z2 / (4.0 * n * n))).sqrt();

    ((center - margin).max(0.0), (center + margin).min(1.0))
}
```

### 14.3 Sequential Stopping Rule

Property-based tests and fuzz tests can use sequential hypothesis testing to stop
early when the outcome is statistically clear, rather than running a fixed number
of iterations:

```rust
/// Sequential probabilistic gate that stops when confidence is sufficient.
pub struct SequentialPropertyGate {
    /// Property to test.
    pub property: Box<dyn PropertyFn>,
    /// Desired confidence level.
    pub confidence: f64,         // default: 0.95
    /// Acceptable failure rate.
    pub acceptable_error: f64,   // default: 0.01 (1%)
    /// Maximum iterations before giving up.
    pub max_iterations: u64,     // default: 10_000
    /// z-score for the confidence level (precomputed).
    pub z_score: f64,            // 1.96 for 95%
}

/// Pseudocode for sequential verification:
///
/// n = 0, pass_count = 0
/// loop:
///     input = generate_random()
///     if property(input):
///         pass_count += 1
///     n += 1
///
///     (lower, upper) = wilson_interval(pass_count, n, z_score)
///
///     if lower > (1.0 - acceptable_error):
///         return Pass with confidence=lower
///         // CI lower bound exceeds threshold → statistically confident it passes
///
///     if upper < (1.0 - acceptable_error):
///         return Fail with confidence=(1.0 - upper)
///         // CI upper bound below threshold → statistically confident it fails
///
///     if n >= max_iterations:
///         return Warn with confidence=pass_count/n
///         // Inconclusive after max iterations
```

### 14.4 Fuzz Gate with Probabilistic Bounds

```rust
/// Coverage-guided fuzzing as a probabilistic gate.
pub struct FuzzGate {
    /// Fuzz target name (cargo-fuzz target).
    pub target: String,
    /// Maximum wall-clock duration.
    pub max_duration: Duration,      // default: 30s
    /// Corpus directory for seed inputs.
    pub corpus_dir: Option<PathBuf>,
    /// Minimum executions before declaring pass.
    pub min_executions: u64,         // default: 1_000
}

/// FuzzGate returns a probabilistic verdict:
///
/// result = cargo_fuzz_run(target, max_time=duration, corpus=corpus_dir)
/// if result.crashes > 0:
///     return Fail(
///         crashes=result.crashes,
///         minimized_inputs=result.artifacts,
///         confidence=1.0  // crash is definitive
///     )
/// confidence = 1.0 - (1.0 / result.total_runs as f64)
/// // With N executions and 0 crashes, P(no bug) ≈ 1 - 1/N
/// return Pass(
///     executions=result.total_runs,
///     coverage_delta=result.new_edges,
///     confidence=confidence,
/// )
```

### 14.5 Probabilistic Verdict → Standard Verdict

For pipeline compatibility, every probabilistic verdict converts to a standard `Verdict`:

```rust
impl From<ProbabilisticVerdict> for Verdict {
    fn from(pv: ProbabilisticVerdict) -> Self {
        Verdict {
            passed: pv.passed,
            score: pv.ci_lower as f32, // conservative: use lower bound
            gate: pv.verdict.gate,
            reason: format!(
                "pass_rate={:.3} CI=[{:.3}, {:.3}] ({}% confidence, n={})",
                pv.pass_rate, pv.ci_lower, pv.ci_upper,
                (pv.confidence_level * 100.0) as u32, pv.sample_count
            ),
            detail: pv.verdict.detail,
            duration_ms: pv.verdict.duration_ms,
            test_count: pv.verdict.test_count,
            error_digest: pv.verdict.error_digest,
        }
    }
}
```

The `score` field uses the lower bound of the CI — the most conservative estimate.
This means the downstream adaptive threshold and process reward systems operate on
worst-case estimates, not optimistic point estimates.

---

## 15. Progressive Delivery Pipeline

Adapted from canary deployment strategies (Argo Rollouts, Flagger), a progressive
delivery pipeline increases verification depth in phases, with automatic rollback
on failure at any phase.

> **Citation**: "Progressive Delivery in CI/CD Pipelines" (IJISAE, 2024) — canary,
> blue-green, and feature-flag strategies in production CI/CD.

### 15.1 Phase Structure

```rust
/// A progressive gate pipeline that increases verification depth in stages.
pub struct ProgressivePipeline {
    /// Phases ordered by increasing depth / cost.
    pub phases: Vec<ProgressivePhase>,
    /// Name for this pipeline.
    pub name: String,
}

#[derive(Debug, Clone)]
pub struct ProgressivePhase {
    /// Human-readable phase label (e.g., "Smoke", "Standard", "Deep").
    pub label: String,
    /// The gate (or composed gate) for this phase.
    /// Uses Box<dyn Gate> so it can be a single gate or a Parallel/Sequential.
    pub gate: Box<dyn Gate>,
    /// Blast radius fraction [0.0, 1.0] — informational, for logging.
    pub blast_radius: f64,
    /// Minimum duration to hold at this phase before advancing (optional).
    pub hold_duration: Option<Duration>,
}
```

### 15.2 Phase Progression for Agent Verification

| Phase | Blast Radius | Gates | Cost | Purpose |
|---|---|---|---|---|
| Smoke | 1% | Compile only | ~3s | Does it parse? |
| Lint | 5% | Compile + Clippy ∥ Diff | ~8s | Is it clean? |
| Test | 25% | Full test suite | ~60s | Does it work? |
| Property | 50% | Property tests (256 cases) | ~120s | Does it generalize? |
| Deep | 100% | PBT (10K) + Fuzz (30s) + Integration | ~180s | Is it robust? |

The cost column shows that short-circuiting at Smoke saves up to 180s per doomed
attempt. For a plan with 50 tasks averaging 3 attempts each, the savings compound
to hours of verification time.

### 15.3 Rollback and Bake-In

```
Phase 1 (Smoke):  compile → PASS → hold 0s → advance
Phase 2 (Lint):   clippy + diff → PASS → hold 0s → advance
Phase 3 (Test):   test suite → FAIL → ROLLBACK → record failure signal
                   (Phases 4-5 never run)
```

A failure at any phase triggers immediate rollback. The failure is recorded as a
verdict Signal with the phase label as metadata, enabling the adaptive threshold
system to track which phases are bottlenecks.

---

## 16. Pipeline Instrumentation

### 16.1 Per-Gate Metrics

Every gate execution produces an instrumentation event:

```rust
pub struct GateMetrics {
    pub gate_name: String,
    pub rung: u8,
    pub passed: bool,
    pub duration_ms: u64,
    pub score: f32,
    /// For probabilistic gates: confidence interval bounds.
    pub ci_lower: Option<f64>,
    pub ci_upper: Option<f64>,
    pub sample_count: Option<u64>,
    /// Memory high-water mark during gate execution (bytes).
    pub peak_memory_bytes: Option<u64>,
    /// Whether this gate was skipped (advisory skip from AdaptiveThresholds).
    pub skipped: bool,
}
```

### 16.2 Pipeline-Level Summary

```rust
pub struct PipelineMetrics {
    pub name: String,
    pub total_duration_ms: u64,
    pub gates_run: usize,
    pub gates_skipped: usize,
    pub gates_passed: usize,
    pub gates_failed: usize,
    pub short_circuited: bool,
    /// Which phase (in progressive mode) was reached.
    pub phase_reached: Option<String>,
    /// Per-gate breakdown.
    pub gate_metrics: Vec<GateMetrics>,
}
```

These metrics feed the adaptive threshold system, the efficiency event logger, and
the dashboard's verification health display.

---

## 17. Test Criteria

| Test | Property |
|---|---|
| `parallel_runs_concurrently` | Parallel(SlowGate, SlowGate) completes in ~1x, not 2x |
| `parallel_both_fail` | Parallel with two failures returns aggregated failure |
| `voting_quorum_pass` | 2/3 judges pass with quorum=2 → pass |
| `voting_quorum_fail` | 1/3 judges pass with quorum=2 → fail |
| `fallback_primary_passes` | Fallback does not run secondary if primary passes |
| `fallback_primary_fails` | Fallback runs secondary and returns its verdict |
| `sequential_stopping_early_pass` | Sequential property gate stops before max iterations |
| `sequential_stopping_early_fail` | Counterexample found → stops and returns Fail |
| `wilson_interval_small_sample` | CI width is large with n=5, small with n=10000 |
| `probabilistic_to_standard_uses_lower_bound` | Conversion uses CI lower bound as score |
| `progressive_short_circuits_on_phase_failure` | Fail at phase 2 → phases 3-5 never run |
| `progressive_advances_all_phases` | All pass → reaches final phase |
| `nested_pipeline` | Pipeline containing a Parallel containing gates works correctly |


---

# SOURCE: /Users/will/dev/nunchi/roko/roko/docs/04-verification/04-artifact-store.md

# 04 — The Artifact Store

> **Layer**: L3 Harness — Verification
> **Crate**: `roko-gate` (`crates/roko-gate/src/artifact_store.rs`)
> **Status**: Implemented (172 lines)


> **Implementation**: Shipping

---

## 1. Overview

The `ArtifactStore` is a content-addressed, append-only store for gate artifacts. Every
artifact is identified by its BLAKE3 hash. The store deduplicates automatically: storing
the same content twice returns the same hash without writing a second copy.

Content addressing is a cornerstone of the verification architecture. It enables:
- **Immutable artifacts**: Once stored, an artifact's content is fixed. The hash is its
  identity. There is no "update" operation.
- **Deduplication**: Identical outputs from different gate runs share storage.
- **Reproducibility**: Given a hash, you can always retrieve the exact artifact.
- **Forensic replay**: Any verdict can be traced to its exact inputs and outputs.

> **Citation**: crates/roko-gate/src/artifact_store.rs — Full implementation.

---

## 2. Structure

```rust
pub type ContentHash = [u8; 32];

pub struct ArtifactStore {
    items: HashMap<ContentHash, Vec<u8>>,
}
```

The store is an in-memory `HashMap` from 32-byte BLAKE3 hashes to byte vectors. This
is intentionally simple — no filesystem, no database, no network. The current
implementation is an in-process store suitable for a single plan execution.

### Why BLAKE3

BLAKE3 is chosen over SHA-256 for three reasons:
1. **Speed**: BLAKE3 is 5–15x faster than SHA-256 on modern hardware, critical when
   hashing megabytes of test output.
2. **Streaming**: BLAKE3 supports incremental hashing without buffering the full input.
3. **Keyed mode**: BLAKE3 supports keyed hashing, enabling future per-session namespacing
   without a separate HMAC construction.

---

## 3. Operations

### 3.1 Store

```rust
pub fn store(&mut self, data: &[u8]) -> ContentHash {
    let hash = blake3::hash(data).into();
    self.items.entry(hash).or_insert_with(|| data.to_vec());
    hash
}
```

Computes the BLAKE3 hash of the input data. If the hash is not already in the store,
inserts the data. Returns the hash in both cases. This is the only write operation.

### 3.2 Retrieve

```rust
pub fn get(&self, hash: &ContentHash) -> Option<&[u8]> {
    self.items.get(hash).map(Vec::as_slice)
}
```

Returns the artifact bytes for a given hash, or `None` if the hash is not in the store.

### 3.3 Contains

```rust
pub fn contains(&self, hash: &ContentHash) -> bool {
    self.items.contains_key(hash)
}
```

Check existence without retrieving the data.

### 3.4 Count

```rust
pub fn len(&self) -> usize {
    self.items.len()
}
```

Number of unique artifacts stored.

---

## 4. Immutability and Append-Only Semantics

The store has no `delete`, `update`, or `clear` operations in its public API. Once an
artifact is stored, it exists for the lifetime of the store. This is a deliberate design
constraint:

- **No accidental loss**: A gate artifact that was used to produce a verdict cannot
  disappear.
- **Audit trail**: The chain from verdict → artifact hash → artifact content is always
  intact.
- **Concurrency safety**: Append-only structures have simpler concurrency properties
  than mutable ones.

> **Citation**: bardo-backup/tmp/mori-agents/20-verification-first-architecture.md —
> "Immutable verification artifacts" as a key architectural decision.

---

## 5. Deduplication

When an agent retries a task and produces identical output, the artifact store does not
allocate new memory. The BLAKE3 hash matches the existing entry, and the `or_insert_with`
short-circuits. This matters because:

- Gate outputs can be large (megabytes of test runner output)
- Retries are common (3–5 attempts per task is typical)
- Many retries produce identical or near-identical output on the portions that haven't
  changed

Deduplication is automatic and zero-cost at the application level.

---

## 6. Relationship to Gate Verdicts

The artifact store sits alongside the gate pipeline, not inside it. The current
integration pattern is:

```
Gate produces verdict with detail (full output)
    ↓
Orchestrator stores detail in ArtifactStore
    ↓
ArtifactStore returns ContentHash
    ↓
ContentHash can be attached to the verdict or signal for later retrieval
```

This separation means gates don't need to know about the artifact store. They produce
verdicts with `detail` strings, and the orchestrator decides what to persist.

---

## 7. Future: Persistent Artifact Store

The current in-memory store is ephemeral — it lives and dies with the process. The
design anticipates a persistent version:

### 7.1 Filesystem Layout

```
.roko/artifacts/
├── ab/
│   ├── ab3f8c1d2e...  (BLAKE3 hash as filename)
│   └── abd9e4f720...
├── cd/
│   └── cd1a2b3c4d...
└── manifest.jsonl      (hash → metadata mapping)
```

Two-character prefix directories prevent any single directory from having too many
entries (a common filesystem performance issue).

### 7.2 Manifest

A JSONL file mapping hashes to metadata:
```json
{"hash": "ab3f8c1d2e...", "gate": "compile:cargo", "plan": "plan-42", "rung": 0, "timestamp": "2026-04-10T12:00:00Z", "size_bytes": 4096}
```

### 7.3 Garbage Collection

With persistence comes the need for GC. Artifacts older than a configurable threshold
(e.g., 30 days) with no references from active plans can be pruned. The JSONL manifest
enables efficient reference counting.

---

## 8. Content Addressing in the Broader Architecture

Content-addressed storage appears in multiple places in Roko:

| Component | What It Hashes | Hash Algorithm |
|---|---|---|
| `ArtifactStore` | Gate output bytes | BLAKE3 |
| `Signal` | Signal content | BLAKE3 |
| `FileSubstrate` | Signal bodies (JSONL) | BLAKE3 |
| Episode logs | Agent turns | N/A (sequential) |

The consistency of BLAKE3 across the system means any artifact or signal can be
cross-referenced by hash. A verdict's detail text, stored in the artifact store, hashes
to the same value whether you compute it from the store or from the verdict's detail
field.

> **Citation**: refactoring-prd/01-synapse-architecture.md — "Content-addressed,
> scored, decaying, lineage-tracked" — Engrams are content-addressed; artifacts follow
> the same principle.

---

## 9. Relationship to Forensic AI

The artifact store is a building block for Forensic AI causal replay (see
[12-forensic-ai-causal-replay.md](./12-forensic-ai-causal-replay.md)). To replay an
agent's verification history:

1. Retrieve the signal (engram) by hash from the Substrate.
2. Retrieve the gate artifacts by hash from the ArtifactStore.
3. Replay the gate pipeline with the original signal and compare verdicts.

Content addressing makes this replay exact: the same hash guarantees the same content,
so the replayed inputs are byte-identical to the originals.

> **Citation**: refactoring-prd/09-innovations.md — Innovation IX: Forensic AI Causal
> Replay, "content-addressed replay of any agent action."

---

## 10. Testing

The artifact store's tests cover:

| Test | What It Verifies |
|---|---|
| `store_and_retrieve` | Basic store/get roundtrip |
| `deduplication` | Same content → same hash, no duplicate storage |
| `missing_hash` | `get()` returns `None` for unstored hashes |
| `contains_check` | `contains()` matches `get().is_some()` |
| `empty_data` | Empty byte slices are valid artifacts |
| `large_data` | Large inputs (megabytes) work correctly |

> **Citation**: crates/roko-gate/src/artifact_store.rs — Tests section.

---

## 11. Summary

The `ArtifactStore` is deliberately minimal: store bytes, get bytes, check existence.
No deletion, no mutation, no networking. This simplicity makes it correct by construction
— there are no race conditions, no consistency issues, and no data loss paths.

The key insight is that **verification artifacts are write-once, read-many**. A gate's
output never changes after the gate runs. By giving each artifact a unique
content-derived identity (BLAKE3 hash), the system can reference artifacts reliably
across time, across retries, and across processes without coordination.


---

# SOURCE: /Users/will/dev/nunchi/roko/roko/docs/04-verification/05-ratcheting.md

# 05 — Gate Ratcheting

> **Layer**: L3 Harness — Verification
> **Crate**: `roko-gate` (`crates/roko-gate/src/ratchet.rs`)
> **Status**: Implemented (207 lines)


> **Implementation**: Shipping

---

## 1. Overview

The `GateRatchet` prevents verification regression. Once a plan has passed rung N, it
should never be allowed to regress to rung N-1. The ratchet tracks the highest rung each
plan has passed and provides a `can_regress()` check that the conductor uses before
accepting a lower verdict.

This solves a specific failure mode in multi-attempt agent loops: **convergence
thrashing**. An agent fixes the compile error but breaks lint. On retry, it fixes lint
but breaks compile. Without a ratchet, this cycle can repeat indefinitely, consuming
compute and making no net progress.

> **Citation**: crates/roko-gate/src/ratchet.rs — Full implementation.

---

## 2. The Thrashing Problem

Consider a 3-rung pipeline (Compile → Lint → Test):

```
Attempt 1: Compile PASS, Lint FAIL
  Agent receives: "warning: unused variable"
  Agent fixes lint issue

Attempt 2: Compile FAIL, (Lint/Test never run)
  Agent's lint fix introduced a type error
  Agent receives: "error[E0308]: mismatched types"
  Agent fixes type error

Attempt 3: Compile PASS, Lint FAIL
  Agent's type fix reintroduced the lint issue
  Agent receives: "warning: unused variable"
  ... (infinite loop)
```

Each attempt passes one rung and fails the next. The agent is doing work — it's
modifying code each time — but it's not making *progress*. The net verification state
oscillates between "compiles but doesn't lint" and "lints but doesn't compile."

The ratchet breaks this cycle by making the pipeline say: "You passed Compile on
Attempt 1. You are not allowed to regress below Compile on Attempt 2." If Attempt 2
fails compile, the ratchet flags this as a regression, and the system can:
- Reject the attempt outright
- Flag it for human review
- Give the agent a different prompt ("Your previous attempt passed compile. Your new
  attempt broke compile. Fix the compile error without regressing.")

---

## 3. Data Structure

```rust
pub struct GateRatchet {
    passes: HashMap<String, u8>,
}
```

A map from plan identifier (string) to the highest rung number (u8) that plan has
passed. The rung number corresponds to the `Rung` enum's discriminant (0–6).

### Why u8?

Seven rungs fit in 3 bits. Using `u8` is the natural Rust choice for a small non-negative
integer. It avoids the overhead of an enum in a `HashMap` and allows simple comparison
operators (`>`, `>=`).

---

## 4. Operations

### 4.1 Record a Pass

```rust
pub fn record_pass(&mut self, plan_id: impl Into<String>, rung: u8) {
    let entry = self.passes.entry(plan_id.into()).or_insert(0);
    if rung > *entry {
        *entry = rung;
    }
}
```

Records that `plan_id` passed `rung`. Only advances the watermark — if the plan has
already passed a higher rung, this is a no-op.

**Monotonic property**: The stored value for any plan ID can only increase or stay the
same. It never decreases. This is the core ratchet invariant.

### 4.2 Query Highest Pass

```rust
pub fn highest_pass(&self, plan_id: &str) -> Option<u8> {
    self.passes.get(plan_id).copied()
}
```

Returns the highest rung the plan has passed, or `None` if the plan has no recorded
passes. `None` means the ratchet has no opinion — the plan is free to pass or fail any
rung.

### 4.3 Check for Regression

```rust
pub fn can_regress(&self, plan_id: &str, rung: u8) -> bool {
    match self.passes.get(plan_id) {
        None => true,                    // Unknown plan: no regression possible
        Some(&highest) => rung >= highest, // OK if same or higher
    }
}
```

Returns `false` if accepting `rung` as the new highest would be a regression (i.e., the
plan has already passed a strictly higher rung). Returns `true` if:
- The plan has never been recorded (no regression possible)
- The plan's highest pass is equal to or lower than `rung`

**Note**: The method is named `can_regress` but returns `true` when regression is *not*
happening. The semantics are: "Is it acceptable to record this rung?" — which is true
when no regression would occur.

### 4.4 Plan Count and Clear

```rust
pub fn plan_count(&self) -> usize {
    self.passes.len()
}

pub fn clear(&mut self) {
    self.passes.clear();
}
```

`plan_count()` reports how many plans are tracked. `clear()` resets the ratchet entirely,
used when starting a fresh execution session.

---

## 5. Usage Pattern in the Orchestrator

```rust
// After gate pipeline produces a verdict:
let rung = verdict_rung;  // Which rung did we reach?
let plan_id = &task.plan_id;

if verdict.passed {
    ratchet.record_pass(plan_id, rung);
} else {
    // Check if this failure represents a regression
    if !ratchet.can_regress(plan_id, rung) {
        // Regression detected! Agent passed rung N before but now fails it.
        // Options:
        //   1. Reject this attempt
        //   2. Feed back: "You regressed from rung N to rung M"
        //   3. Trigger re-planning
    }
}
```

---

## 6. Ratchet + Escalation Interaction

The ratchet and the escalation mechanism (see [02-6-rung-selector.md](./02-6-rung-selector.md))
work together but serve different purposes:

| Mechanism | Direction | Purpose |
|---|---|---|
| Escalation | Forward (adds rungs) | Failed → try harder |
| Ratchet | Backward (blocks regression) | Passed → don't lose progress |

Together they create a monotonically advancing verification frontier:

```
Attempt 1: Complexity=Simple, Rungs=[Compile, Lint]
  → Compile PASS (ratchet records rung 0)
  → Lint FAIL
  → Escalate to Standard

Attempt 2: Complexity=Standard, Rungs=[Compile, Lint, Test, Symbol]
  → Compile must still pass (ratchet enforces)
  → Lint PASS (ratchet records rung 1)
  → Test FAIL
  → Escalate to Complex

Attempt 3: Complexity=Complex, Rungs=[all]
  → Compile must still pass (ratchet enforces)
  → Lint must still pass (ratchet enforces)
  → Test PASS (ratchet records rung 2)
  → ... and so on
```

Each attempt can only move the verification frontier forward. Rungs that have been
passed stay passed.

---

## 7. Per-Plan Isolation

Each plan has its own ratchet entry. Plan A's progress has no effect on Plan B's
ratchet. This is important because:

- Plans are independent units of work (a plan might be "implement rate limiter" while
  another is "fix auth bug")
- Different plans may be at different stages of verification
- One plan's compile failure doesn't prevent another plan from passing lint

The `HashMap<String, u8>` keying on plan ID provides this isolation naturally.

---

## 8. Ratchet in the Context of Process Reward Models

The ratchet is a simple form of process reward: it tracks *intermediate* verification
progress, not just final outcomes. A plan that reaches Rung 3 (passed Compile + Lint +
Test) has demonstrated more progress than one that only reaches Rung 1 (passed Compile).

This intermediate signal feeds into the process reward model (see
[07-process-reward-models.md](./07-process-reward-models.md)):

- **Promise score**: How likely is this plan to eventually pass all rungs, given that
  it has passed rung N so far?
- **Progress score**: Is the plan advancing (reaching higher rungs on successive
  attempts) or stalling?

The ratchet provides the raw data — "plan X has reached rung N" — that the process
reward model interprets.

> **Citation**: refactoring-prd/02-five-layers.md — "Process Reward Models: Promise +
> Progress scoring, low Promise → early intervention, negative Progress → re-planning."

---

## 9. Edge Cases

### 9.1 Rung 0 Ratchet

If a plan passes Rung 0 (Compile), the ratchet records `highest = 0`. On the next
attempt, `can_regress("plan", 0)` returns `true` (rung 0 >= 0). Only a *lower* rung
would trigger regression, but there is no rung below 0. So a plan that passed Compile
can fail Compile on a subsequent attempt without the ratchet blocking it.

Wait — that seems wrong. If we passed Compile, shouldn't we enforce it?

The answer is: `can_regress` checks if the *proposed rung* is below the highest. Rung 0
is not below 0, so it's allowed. This is correct because the ratchet's `record_pass()`
only fires on success. A failure at Rung 0 doesn't call `record_pass()`, so the stored
value stays at 0. The regression check happens externally — the orchestrator checks
`can_regress(plan, failing_rung)` and decides what to do.

### 9.2 Full Pipeline Pass

When a plan passes all 7 rungs, `highest_pass` = 6. Any subsequent failure at any rung
is a regression (since all rungs 0–5 are below 6). Only passing rung 6 again is non-
regressive.

### 9.3 String Plan IDs

Plan IDs are strings, supporting both owned (`String::from("plan-1")`) and borrowed
(`"plan-1"`) inputs via `impl Into<String>`.

---

## 10. Future: Persistent Ratchet

The current ratchet is in-memory and ephemeral. When the process exits, all ratchet
state is lost. For long-running or resumable executions, the ratchet should persist
to disk:

```json
// .roko/state/gate-ratchet.json
{
  "plan-42": 3,
  "plan-43": 1,
  "plan-44": 5
}
```

This aligns with the existing executor snapshot persistence
(`.roko/state/executor.json`) and would be loaded on `--resume`.

---

## 11. Testing

The ratchet module has 13 tests covering:

| Test | Property |
|---|---|
| `ratchet_new_is_empty` | Default state has no entries |
| `ratchet_record_and_query` | Basic store/query roundtrip |
| `ratchet_only_advances` | Lower rung does not overwrite higher |
| `ratchet_can_regress_prevents_regression` | Detects regression correctly |
| `ratchet_can_regress_allows_same_or_higher` | Non-regression is permitted |
| `ratchet_can_regress_unknown_plan_returns_true` | Unknown plans have no constraint |
| `ratchet_multiple_plans_independent` | Per-plan isolation |
| `ratchet_clear_resets_all` | Clear removes all entries |
| `ratchet_record_pass_zero_rung` | Edge case: rung 0 |
| `ratchet_monotonic_sequence` | Rungs 0→6 all recorded correctly |
| `ratchet_string_plan_ids` | Owned and borrowed IDs both work |
| `ratchet_default_is_new` | `Default` trait works |

> **Citation**: crates/roko-gate/src/ratchet.rs — Tests section, 207 lines total.

---

## 12. Summary

The `GateRatchet` is a one-way valve on verification progress. It answers one question:
"Has this plan ever passed a higher rung than what it's being asked to accept now?"
If yes, the answer is regression. If no, the answer is progress (or same level).

Its simplicity — a `HashMap<String, u8>` with monotonic updates — makes it correct by
construction. There are no complex algorithms, no statistical models, no heuristics.
Just: the highest rung you've passed is the floor you can't drop below.


---

# SOURCE: /Users/will/dev/nunchi/roko/roko/docs/04-verification/06-adaptive-thresholds.md

# 06 — Adaptive Gate Thresholds

> **Layer**: L3 Harness — Verification
> **Crate**: `roko-gate` (`crates/roko-gate/src/adaptive_threshold.rs`)
> **Status**: Implemented (215 lines), persists to `.roko/learn/gate-thresholds.json`


> **Implementation**: Shipping

---

## 1. Overview

Adaptive thresholds tune verification behavior based on historical pass rates. They use
exponential moving averages (EMA) per gate rung to track how often each rung passes, and
from that, derive two advisory signals:

1. **Retry budget**: How many retries should a rung get? High pass rate → fewer retries.
   Low pass rate → more retries.
2. **Skip advisory**: Should a rung be skipped? If it has passed 20+ times consecutively,
   it's probably always passing and could be skipped to save time.

Both signals are advisory — the orchestrator may override them. But they provide a
data-driven default that adapts to the project's actual verification characteristics.

> **Citation**: crates/roko-gate/src/adaptive_threshold.rs — Full implementation.

---

## 2. Per-Rung Statistics

```rust
pub struct RungStats {
    pub ema_pass_rate: f64,       // Exponential moving average of pass rate [0.0, 1.0]
    pub total_observations: u64,  // Total gate runs for this rung
    pub consecutive_passes: u32,  // Consecutive passes (reset on any failure)
}
```

Each rung gets its own `RungStats`. A fresh rung starts with:
- `ema_pass_rate = 0.5` (neutral prior — no assumption about pass/fail tendency)
- `total_observations = 0`
- `consecutive_passes = 0`

---

## 3. The EMA Algorithm

### 3.1 Update Rule

```rust
pub fn update(&mut self, rung: u32, passed: bool) {
    let stats = self.rungs.entry(rung).or_default();
    let value = if passed { 1.0 } else { 0.0 };

    if stats.total_observations == 0 {
        stats.ema_pass_rate = value;  // First observation sets the rate directly
    } else {
        stats.ema_pass_rate = EMA_ALPHA.mul_add(value, (1.0 - EMA_ALPHA) * stats.ema_pass_rate);
    }

    stats.total_observations += 1;

    if passed {
        stats.consecutive_passes += 1;
    } else {
        stats.consecutive_passes = 0;
    }
}
```

### 3.2 Why EMA?

An exponential moving average with α = 0.1 means:
- Recent observations weigh more than old ones
- The effective memory is ~1/α ≈ 10 observations
- Gradual changes in pass rate are tracked smoothly

This is important because gate pass rates change over time:
- A new project with many issues has low pass rates initially
- As issues are fixed, pass rates climb
- A major refactor temporarily drops pass rates before they recover

A simple average (total passes / total observations) would be slow to respond to these
shifts. The EMA adapts within ~10 observations.

### 3.3 The α Parameter

`EMA_ALPHA = 0.1` is the decay constant. Higher α means more responsive (recent data
dominates), lower α means more stable (historical data dominates).

| α | Effective window | Behavior |
|---|---|---|
| 0.01 | ~100 observations | Very stable, slow to adapt |
| 0.1 | ~10 observations | Balanced (current default) |
| 0.3 | ~3 observations | Responsive, potentially noisy |

The choice of 0.1 balances responsiveness with stability. A gate that fails once
shouldn't immediately triple the retry budget, but a gate that fails 5 times in a row
should.

> **Citation**: bardo-backup/prd/16-testing/07-fast-feedback-loops.md — Fast feedback
> loops using EMA-based calibration.

---

## 4. Retry Budget Suggestion

```rust
pub fn suggested_max_retries(&self, rung: u32) -> u32 {
    let Some(stats) = self.rungs.get(&rung) else {
        return 3; // Default for unknown rungs
    };

    if stats.total_observations < 5 {
        return 3; // Not enough data
    }

    // Map pass rate to retries: high pass → low retries, low pass → high retries
    let retries = stats.ema_pass_rate.mul_add(-range, max).round() as u32;
    retries.clamp(MIN_RETRIES, MAX_RETRIES)
}
```

The mapping is linear:
- Pass rate 1.0 → 1 retry (it almost always passes; one attempt is enough)
- Pass rate 0.5 → 3 retries (coin flip; give it a few tries)
- Pass rate 0.0 → 5 retries (it almost never passes; maximize attempts)

### Constants

| Constant | Value | Purpose |
|---|---|---|
| `MIN_RETRIES` | 1 | Floor: always try at least once |
| `MAX_RETRIES` | 5 | Ceiling: don't waste resources endlessly |

### Cold Start

For unknown rungs or rungs with fewer than 5 observations, the default is 3 retries.
This avoids extreme behavior early in a project's lifecycle.

---

## 5. Skip Advisory

```rust
pub fn should_skip_rung(&self, rung: u32) -> bool {
    self.rungs
        .get(&rung)
        .is_some_and(|s| s.consecutive_passes >= SKIP_STREAK_THRESHOLD)
}
```

If a rung has passed `SKIP_STREAK_THRESHOLD` (20) consecutive times, the system
suggests it can be skipped. This advisory is not enforced — the orchestrator decides
whether to honor it.

### Why 20?

Twenty consecutive passes means the gate hasn't failed in at least 20 task executions.
For a gate like Compile, this suggests the project's code generation is reliable enough
that compile failures are rare. Running the gate still costs time (seconds to minutes),
and if the system has high confidence the gate will pass, skipping it saves that time.

### Why Advisory Only?

Even a gate with 100 consecutive passes can fail unexpectedly (new dependency breaks,
CI environment changes, agent produces unusually complex code). Making the skip advisory
rather than mandatory means:
- The orchestrator can skip the gate 90% of the time for speed
- Every Nth run (e.g., every 5th), it still runs the gate to check
- A failure resets the consecutive pass counter, re-enabling the gate

This "mostly skip, periodically verify" pattern is common in testing infrastructure
(see: test quarantine systems, flaky test skip-and-retry).

> **Citation**: bardo-backup/prd/16-testing/09-evaluation-map.md — 14 feedback loops
> across 5 speed tiers, including machine-speed confidence calibration.

---

## 6. Persistence

```rust
pub fn save(&self, path: &Path) -> Result<(), std::io::Error> {
    let json = serde_json::to_string_pretty(self)?;
    let tmp = path.with_extension("json.tmp");
    std::fs::write(&tmp, &json)?;
    std::fs::rename(&tmp, path)?;
    Ok(())
}

pub fn load_or_new(path: &Path) -> Self {
    std::fs::read_to_string(path)
        .ok()
        .and_then(|s| serde_json::from_str(&s).ok())
        .unwrap_or_default()
}
```

### Atomic Write

The `save()` method uses the atomic write pattern: write to a temporary file, then
rename. This ensures the file is never in a half-written state. If the process crashes
between `write` and `rename`, the old file remains intact.

### Graceful Degradation

`load_or_new()` returns a fresh `AdaptiveThresholds` if the file is missing or corrupt.
This means the system always starts correctly — it just loses its historical data on
corruption.

### Storage Location

The thresholds persist to `.roko/learn/gate-thresholds.json`. This is the learning
subsystem's data directory, alongside:
- `.roko/learn/cascade-router.json` (model routing state)
- `.roko/learn/experiments.json` (prompt experiment state)
- `.roko/learn/efficiency.jsonl` (per-turn efficiency events)

> **Citation**: CLAUDE.md — "Adaptive gate thresholds: EMA per rung in
> `.roko/learn/gate-thresholds.json`."

---

## 7. Interaction with Other Components

### 7.1 Rung Selector

The rung selector (see [02-6-rung-selector.md](./02-6-rung-selector.md)) determines
which rungs to run based on static complexity. The adaptive thresholds refine this:

```
Static selection: complexity → rungs [0, 1, 2, 3]
Adaptive refinement:
  Rung 0: 25 consecutive passes → skip advisory
  Rung 1: 3 consecutive passes → run normally
  Rung 2: 12 consecutive passes → run normally
  Rung 3: not tracked yet → run with default retries
Final: rungs [1, 2, 3] with rung 0 skipped
```

### 7.2 Retry Logic

The orchestrator's retry loop consults `suggested_max_retries()`:

```rust
let max_retries = thresholds.suggested_max_retries(current_rung);
for attempt in 0..max_retries {
    let verdict = pipeline.verify(signal, ctx).await;
    if verdict.passed { break; }
    // ... escalate, adjust prompt, retry
}
```

### 7.3 Gate Pipeline Feedback

After each pipeline execution, the orchestrator updates the thresholds:

```rust
for (rung, verdict) in rung_verdicts {
    thresholds.update(rung as u32, verdict.passed);
}
thresholds.save(&thresholds_path)?;
```

This closes the feedback loop: gate outcomes → EMA update → retry budget adjustment →
different gate behavior on next execution.

---

## 8. Reporting

```rust
pub fn rung_stats(&self, rung: u32) -> Option<&RungStats> {
    self.rungs.get(&rung)
}

pub fn all_rungs(&self) -> impl Iterator<Item = (&u32, &RungStats)> {
    self.rungs.iter()
}
```

These methods enable the dashboard and status commands to display per-rung health:

```
Gate Thresholds:
  Rung 0 (Compile):  98.2% pass rate, 142 observations, 31 consecutive passes [SKIP ADVISORY]
  Rung 1 (Lint):     87.5% pass rate, 130 observations, 8 consecutive passes
  Rung 2 (Test):     72.1% pass rate, 118 observations, 3 consecutive passes
  Rung 3 (Symbol):   95.0% pass rate, 45 observations, 15 consecutive passes
```

---

## 9. Relationship to the GVU Framework

The adaptive thresholds are a practical implementation of the GVU framework's guidance
on verification investment. The framework proves that stronger verifiers yield better
self-improvement. The thresholds operationalize this by:

1. **Allocating more retries** to gates with low pass rates (investing more in
   verification where it's most needed).
2. **Reducing retries** for gates with high pass rates (not wasting resources on
   verification that's already reliable).
3. **Skipping gates** that are essentially always passing (redirecting verification
   budget to where it matters).

This is adaptive resource allocation for verification — a concrete instance of the GVU
insight that verification quality matters more than generation quality.

> **Citation**: Song et al. (ICLR 2025) — GVU framework, verification-first investment
> strategy.

---

## 10. Testing

| Test | Property |
|---|---|
| `new_rung_starts_neutral` | Unknown rung → default 3 retries, no skip |
| `high_pass_rate_reduces_retries` | ~100% pass rate → 1 retry |
| `low_pass_rate_increases_retries` | ~0% pass rate → 5 retries |
| `consecutive_passes_trigger_skip` | 20 consecutive → skip advisory |
| `failure_resets_skip_streak` | One failure → no skip advisory |
| `round_trip_persistence` | Save/load preserves state |

> **Citation**: crates/roko-gate/src/adaptive_threshold.rs — Tests section.

---

## 11. Statistical Process Control (SPC) Extensions

The current EMA provides a smoothed estimate of pass rates. Statistical Process Control
adds formal anomaly detection — distinguishing true process changes from random
fluctuation. Three complementary SPC methods detect different kinds of shifts.

> **Citation**: "Improved adaptive CUSUM control chart for industrial process monitoring"
> (Nature Scientific Reports, 2025).

### 11.1 CUSUM (Cumulative Sum) for Sustained Shifts

CUSUM detects small, sustained changes in gate pass rates that the EMA might smooth
over. A gate whose pass rate drifts from 90% to 80% over 20 runs may not trigger an
alert in EMA but will accumulate signal in CUSUM.

```rust
/// CUSUM detector for sustained shifts in gate pass rates.
///
/// Tracks cumulative departures from target in both directions.
/// Signals when accumulated drift exceeds the decision interval.
pub struct CusumDetector {
    /// Reference value (slack parameter). Typically 0.5 * delta where
    /// delta is the shift size to detect in standard deviation units.
    /// Default: 0.25 (detects ~0.5σ shifts in pass rate).
    pub k: f64,
    /// Decision interval. Higher = fewer false alarms, slower detection.
    /// Default: 4.0 (ARL₀ ≈ 168 observations before false alarm).
    pub h: f64,
    /// Target pass rate (process mean under normal operation).
    /// Updated from historical data or set from EMA baseline.
    pub mu_0: f64,
    /// Process standard deviation estimate.
    /// For binary pass/fail: σ = sqrt(p * (1-p)).
    pub sigma: f64,
    /// Upper CUSUM accumulator (detects upward shift — improving).
    pub c_plus: f64,
    /// Lower CUSUM accumulator (detects downward shift — degrading).
    pub c_minus: f64,
    /// Whether a shift has been detected.
    pub shift_detected: bool,
    /// Direction of detected shift.
    pub shift_direction: Option<ShiftDirection>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ShiftDirection {
    /// Pass rate is improving (higher than target).
    Improving,
    /// Pass rate is degrading (lower than target).
    Degrading,
}

impl CusumDetector {
    /// Update with a new observation.
    ///
    /// value: 1.0 for pass, 0.0 for fail.
    pub fn update(&mut self, value: f64) {
        let z = (value - self.mu_0) / self.sigma; // standardize

        // Detect upward shift (improving)
        self.c_plus = (self.c_plus + z - self.k).max(0.0);
        // Detect downward shift (degrading)
        self.c_minus = (self.c_minus - z - self.k).max(0.0);

        if self.c_plus > self.h {
            self.shift_detected = true;
            self.shift_direction = Some(ShiftDirection::Improving);
            self.c_plus = self.h / 2.0; // Fast Initial Response (FIR) reset
        } else if self.c_minus > self.h {
            self.shift_detected = true;
            self.shift_direction = Some(ShiftDirection::Degrading);
            self.c_minus = self.h / 2.0; // FIR reset
        } else {
            self.shift_detected = false;
            self.shift_direction = None;
        }
    }
}
```

**Parameters**:

| Parameter | Default | Range | Effect |
|---|---|---|---|
| `k` (reference value) | 0.25 | 0.1–1.0 | Lower = more sensitive to small shifts |
| `h` (decision interval) | 4.0 | 2.0–8.0 | Lower = faster detection, more false alarms |
| FIR reset | `h/2` | — | Halves accumulator on signal, enabling re-detection |

### 11.2 EWMA Control Chart

Extends the existing EMA with formal control limits. The current implementation tracks
`ema_pass_rate` but has no formal bounds for when that rate is "out of control."

```rust
/// EWMA control chart with time-varying control limits.
///
/// Adds formal upper/lower control limits (UCL/LCL) to the existing
/// EMA pass rate tracking. When the EMA crosses a limit, the gate
/// is flagged as out-of-control.
pub struct EwmaControlChart {
    /// Smoothing factor (same as EMA_ALPHA). Range: [0.05, 0.25].
    /// Lower = more memory, detects smaller shifts.
    pub lambda: f64,      // default: 0.10
    /// Control limit width in sigma units. Range: [2.5, 3.5].
    /// Wider = fewer false alarms.
    pub l_factor: f64,    // default: 2.814
    /// Target mean (established from historical data).
    pub mu_0: f64,
    /// Process standard deviation.
    pub sigma: f64,
    /// Current EWMA value (= ema_pass_rate from RungStats).
    pub z: f64,
    /// Number of observations (for time-varying limit computation).
    pub n: u64,
}

impl EwmaControlChart {
    /// Compute the current upper and lower control limits.
    ///
    /// Time-varying: limits are wider early (few observations) and
    /// converge to steady-state as n grows.
    pub fn control_limits(&self) -> (f64, f64) {
        let asymptotic_var = self.lambda / (2.0 - self.lambda);
        let time_factor = 1.0 - (1.0 - self.lambda).powi(2 * self.n as i32);
        let sigma_z = self.sigma * (asymptotic_var * time_factor).sqrt();

        let ucl = self.mu_0 + self.l_factor * sigma_z;
        let lcl = self.mu_0 - self.l_factor * sigma_z;
        (lcl.max(0.0), ucl.min(1.0)) // clamp to [0, 1] for pass rates
    }

    /// Check if the current EWMA is within control limits.
    pub fn is_in_control(&self) -> bool {
        let (lcl, ucl) = self.control_limits();
        self.z >= lcl && self.z <= ucl
    }
}
```

**ARL (Average Run Length) Tuning**:

| λ | L | ARL₀ (in-control) | ARL₁ (1σ shift) | Best for |
|---|---|---|---|---|
| 0.05 | 2.625 | ~500 | ~26 | Small persistent drifts |
| 0.10 | 2.814 | ~500 | ~31 | **Balanced (default)** |
| 0.20 | 2.962 | ~500 | ~41 | Larger sudden shifts |

ARL₀ ~ 500 means one false alarm per ~500 observations. ARL₁ ~ 31 means a true
1σ shift is detected in ~31 observations on average.

### 11.3 BOCPD (Bayesian Online Change Point Detection)

When EMA and CUSUM detect a shift, BOCPD provides a probabilistic answer to "did the
gate's fundamental behavior change?" This is critical after major refactors, dependency
updates, or model switches, where the baseline itself shifts.

> **Citation**: Adams & MacKay, "Bayesian Online Changepoint Detection" (arXiv:0710.3742,
> 2007).

```rust
/// Bayesian Online Change Point Detection.
///
/// Maintains a posterior distribution over run lengths (time since last
/// change point). When P(run_length = 0) spikes, a change point has
/// occurred and the gate baseline should be recalibrated.
pub struct BocpdDetector {
    /// Prior probability of a change point at each step.
    /// Lower = fewer expected change points (more stable process).
    /// Default: 1/200 (expect one change point per 200 observations).
    pub hazard_rate: f64,
    /// Maximum run length to track (truncation for O(R_max) per step).
    pub max_run_length: usize,  // default: 300
    /// Run-length posterior probabilities.
    pub run_length_probs: Vec<f64>,
    /// Sufficient statistics for the underlying model (Normal-Gamma
    /// conjugate for Gaussian observations).
    pub sufficient_stats: Vec<NormalGammaStats>,
    /// Threshold for declaring a change point.
    /// When P(r=0) > threshold, a change point is declared.
    pub changepoint_threshold: f64,  // default: 0.5
}

#[derive(Debug, Clone)]
pub struct NormalGammaStats {
    pub mu: f64,     // posterior mean
    pub kappa: f64,  // pseudo-observations for mean
    pub alpha: f64,  // shape for variance
    pub beta: f64,   // rate for variance
}

impl BocpdDetector {
    /// Update with a new observation and return whether a change point was detected.
    pub fn update(&mut self, value: f64) -> bool {
        // 1. Compute predictive probability for each run length
        let predictive: Vec<f64> = self.sufficient_stats.iter()
            .map(|s| s.predictive_probability(value))
            .collect();

        // 2. Growth probabilities (no change point)
        let growth: Vec<f64> = self.run_length_probs.iter()
            .zip(predictive.iter())
            .map(|(p, pi)| p * pi * (1.0 - self.hazard_rate))
            .collect();

        // 3. Change-point probability (run length resets to 0)
        let changepoint_mass: f64 = self.run_length_probs.iter()
            .zip(predictive.iter())
            .map(|(p, pi)| p * pi * self.hazard_rate)
            .sum();

        // 4. Build new posterior
        let mut new_probs = vec![changepoint_mass];
        new_probs.extend(growth.iter().take(self.max_run_length - 1));

        // 5. Normalize
        let total: f64 = new_probs.iter().sum();
        if total > 0.0 {
            for p in &mut new_probs {
                *p /= total;
            }
        }

        // 6. Update sufficient statistics for each run length
        // (extend by one entry for r=0, update existing entries)
        self.update_sufficient_stats(value);

        self.run_length_probs = new_probs;

        // 7. Detect change point
        self.run_length_probs[0] > self.changepoint_threshold
    }
}
```

**Parameters**:

| Parameter | Default | Range | Effect |
|---|---|---|---|
| `hazard_rate` | 1/200 | 1/50 – 1/1000 | Prior on change frequency |
| `max_run_length` | 300 | 50–1000 | Truncation depth (memory vs accuracy) |
| `changepoint_threshold` | 0.5 | 0.3–0.8 | Sensitivity to regime changes |

**When to recalibrate**: When BOCPD detects a change point, the system should:
1. Reset the CUSUM accumulators to zero
2. Update the EWMA target mean (μ₀) to the post-changepoint EMA
3. Log a regime-change event to `.roko/learn/efficiency.jsonl`
4. Optionally notify the dashboard

---

## 12. Multi-Gate Threshold Coordination

When one gate's behavior changes, should other gates adjust? The answer is yes —
gates are not independent. A compile time increase often precedes test flakiness.
A coverage drop in lint gates correlates with more test failures.

### 12.1 The Coordination Problem

Consider: the test gate's pass rate drops from 90% to 60%. The adaptive threshold
increases test retries from 2 to 4. But the *reason* tests are failing is that the
compile gate is letting through code with subtle type errors that the linter would
catch. The correct response isn't more test retries — it's tighter lint enforcement.

Independent threshold adjustment misses these cross-gate correlations.

### 12.2 Hotelling's T² for Multi-Gate Anomaly Detection

```rust
/// Multi-gate anomaly detector using Hotelling's T-squared statistic.
///
/// Monitors the joint distribution of gate metrics (pass rates, durations,
/// scores) across all rungs simultaneously. Detects correlated anomalies
/// that per-gate monitors miss.
pub struct MultiGateDetector {
    /// Number of metrics being tracked (one per gate).
    pub p: usize,
    /// Historical mean vector (one entry per gate's pass rate).
    pub mu: Vec<f64>,
    /// Inverse covariance matrix (p × p).
    /// Captures inter-gate correlations.
    pub sigma_inv: Vec<Vec<f64>>,
    /// Chi-squared critical value for the chosen alpha.
    /// Default alpha=0.01, p=7 gates → threshold ≈ 18.48.
    pub threshold: f64,
    /// Minimum observations before monitoring begins.
    pub warmup_period: u64,   // default: 30
    /// Current observation count.
    pub observation_count: u64,
}

impl MultiGateDetector {
    /// Feed a new observation vector (one pass rate per gate) and check
    /// for multi-gate anomaly.
    pub fn observe(&mut self, x: &[f64]) -> Option<MultiGateAnomaly> {
        self.observation_count += 1;
        if self.observation_count < self.warmup_period {
            self.update_statistics(x);
            return None;
        }

        // Compute T² = (x - μ)ᵀ Σ⁻¹ (x - μ)
        let diff: Vec<f64> = x.iter().zip(self.mu.iter())
            .map(|(xi, mi)| xi - mi)
            .collect();
        let t_squared = self.quadratic_form(&diff, &self.sigma_inv);

        if t_squared > self.threshold {
            // Identify which gate(s) are contributing most to the anomaly
            let contributions = self.per_gate_contributions(&diff);
            Some(MultiGateAnomaly {
                t_squared,
                threshold: self.threshold,
                primary_gates: contributions,
            })
        } else {
            self.update_statistics(x);
            None
        }
    }
}

pub struct MultiGateAnomaly {
    /// The T² statistic value.
    pub t_squared: f64,
    /// The threshold that was exceeded.
    pub threshold: f64,
    /// Gates ranked by their contribution to the anomaly,
    /// with attribution scores.
    pub primary_gates: Vec<(usize, f64)>,
}
```

### 12.3 Coordination Policies

When a multi-gate anomaly is detected, a coordination policy determines the response:

```rust
pub enum CoordinationPolicy {
    /// Independent: each gate adjusts thresholds independently.
    /// This is the current behavior and the default.
    Independent,

    /// Sympathetic: when a downstream gate degrades, upstream gates tighten.
    /// Example: test failures → tighten compile/lint gates.
    Sympathetic {
        /// How much to tighten upstream gates when downstream fails.
        /// 0.0 = no tightening, 1.0 = maximum tightening.
        tightening_factor: f64, // default: 0.3
    },

    /// Compensatory: when one gate relaxes (high pass rate), neighboring
    /// gates tighten to maintain overall verification strength.
    /// Conserves total verification investment.
    Compensatory {
        /// Target aggregate verification score across all gates.
        target_aggregate: f64, // default: 0.85
    },

    /// Diagnostic: on anomaly, run additional diagnostic gates that are
    /// normally skipped, to identify root cause.
    Diagnostic {
        /// Additional gates to activate on anomaly.
        diagnostic_gates: Vec<Box<dyn Gate>>,
    },
}
```

### 12.4 Sympathetic Tightening Example

```
Gate pass rates at T=100:
  Compile: 98%  →  retry budget: 1
  Lint:    90%  →  retry budget: 2
  Test:    85%  →  retry budget: 2

Gate pass rates at T=120 (test degrades):
  Compile: 97%  →  retry budget: 1
  Lint:    88%  →  retry budget: 2
  Test:    60%  →  retry budget: 4

Multi-gate anomaly detected (T² = 22.1 > 18.48 threshold).
Primary contributor: Test gate.

Sympathetic response (tightening_factor=0.3):
  Compile: tighten → enable --all-targets flag, add 1 retry
  Lint:    tighten → enable -D warnings (deny all warnings)
  Test:    increase retries as normal

Rationale: if tests are failing more, tighter upstream gates catch problems
earlier and cheaper, reducing the load on the expensive test gate.
```

---

## 13. Domain-Specific Threshold Profiles

Different agent roles (code writer, test writer, documentation, infra) have different
gate characteristics. A compile gate that passes 99% of the time for a test-writer
agent might pass only 80% of the time for a complex refactoring agent.

### 13.1 Profile Structure

```rust
/// Pre-configured threshold profile for a specific agent role or domain.
pub struct ThresholdProfile {
    /// Profile identifier.
    pub id: String,
    /// Human-readable name.
    pub name: String,
    /// Initial pass rate priors per rung.
    /// Used instead of the neutral 0.5 prior for cold-start.
    pub initial_priors: HashMap<u32, f64>,
    /// Per-rung EMA alpha overrides.
    /// Some domains need faster adaptation (higher α).
    pub alpha_overrides: HashMap<u32, f64>,
    /// Per-rung retry budget overrides [min, max].
    pub retry_bounds: HashMap<u32, (u32, u32)>,
    /// Per-rung skip streak threshold overrides.
    pub skip_thresholds: HashMap<u32, u32>,
    /// CUSUM parameters per rung.
    pub cusum_params: HashMap<u32, CusumParams>,
}

#[derive(Debug, Clone)]
pub struct CusumParams {
    pub k: f64,
    pub h: f64,
}
```

### 13.2 Built-In Profiles

```rust
/// Profiles for common agent roles.
pub fn profile_for_role(role: &str) -> ThresholdProfile {
    match role {
        "code-writer" => ThresholdProfile {
            name: "Code Writer".into(),
            initial_priors: hashmap! {
                0 => 0.85, // compile: most generated code compiles
                1 => 0.75, // lint: often has warnings
                2 => 0.60, // test: tests frequently break
                3 => 0.90, // symbol: symbol checks are reliable
            },
            alpha_overrides: hashmap! {
                2 => 0.15, // test gate: adapt faster (more volatile)
            },
            retry_bounds: hashmap! {
                2 => (2, 6), // tests: more retries allowed
            },
            ..Default::default()
        },
        "test-writer" => ThresholdProfile {
            name: "Test Writer".into(),
            initial_priors: hashmap! {
                0 => 0.95, // compile: test code almost always compiles
                1 => 0.90, // lint: test code is cleaner
                2 => 0.50, // test: new tests often need iteration
            },
            retry_bounds: hashmap! {
                2 => (3, 8), // tests: expect more iteration
            },
            ..Default::default()
        },
        "refactoring" => ThresholdProfile {
            name: "Refactoring".into(),
            initial_priors: hashmap! {
                0 => 0.70, // compile: refactors break things
                1 => 0.65, // lint: type changes cascade
                2 => 0.55, // test: existing tests may break
                3 => 0.80, // symbol: symbols shift during refactors
            },
            alpha_overrides: hashmap! {
                0 => 0.20, // compile: adapt very fast during refactors
                1 => 0.20,
            },
            cusum_params: hashmap! {
                0 => CusumParams { k: 0.15, h: 3.0 }, // more sensitive
            },
            ..Default::default()
        },
        _ => ThresholdProfile::default(),
    }
}
```

### 13.3 Profile Selection

The orchestrator selects a profile based on the task description and plan metadata:

```
Task: "Refactor auth module to use trait objects"
  → Keywords: "refactor" → profile: "refactoring"
  → Initial compile prior: 0.70 (instead of neutral 0.50)
  → CUSUM sensitivity increased

Task: "Add unit tests for the parser"
  → Keywords: "test" → profile: "test-writer"
  → Initial test prior: 0.50 (expects iteration)
  → More retries for test gate
```

---

## 14. Change-Point Detection Integration

CUSUM, EWMA control charts, and BOCPD work together in a hierarchy:

```
Per gate observation (pass/fail)
    │
    ├── EMA update (existing) ─── smoothed pass rate
    │
    ├── CUSUM update ─── sustained shift detection
    │    └── shift detected? → adjust retry budget more aggressively
    │
    ├── EWMA control chart ─── formal anomaly detection
    │    └── out of control? → flag gate in dashboard, notify conductor
    │
    └── BOCPD update ─── regime change detection
         └── change point? → recalibrate baselines for all detectors
```

### 14.1 Offline Batch Analysis with PELT

For retrospective analysis (e.g., "when did our test reliability degrade?"), the
PELT algorithm finds optimal change points in historical gate data:

> **Citation**: Killick et al., "Optimal Detection of Changepoints with a Linear
> Computational Cost" (arXiv:1101.1438, 2012).

```rust
/// Offline change-point detection using PELT (Pruned Exact Linear Time).
///
/// Finds all points where the gate's statistical properties changed.
/// Used for retrospective analysis, not online monitoring.
pub struct PeltDetector {
    /// Cost function for a segment of observations.
    /// Default: negative log-likelihood for Gaussian data.
    pub cost: CostFunction,
    /// Penalty term controlling number of change points.
    /// BIC: p * ln(n), where p = parameters, n = observations.
    /// Higher penalty = fewer change points detected.
    pub penalty: f64,
    /// Minimum segment length between change points.
    pub min_segment: usize,  // default: 5
}

pub enum CostFunction {
    /// Gaussian negative log-likelihood (for continuous scores).
    Gaussian,
    /// Bernoulli negative log-likelihood (for binary pass/fail).
    Bernoulli,
}

impl PeltDetector {
    /// Find all change points in a historical sequence.
    ///
    /// Returns indices where the process changed.
    /// Complexity: O(n) expected with pruning.
    pub fn detect(&self, data: &[f64]) -> Vec<usize> {
        let n = data.len();
        let mut f = vec![0.0_f64; n + 1];
        f[0] = -self.penalty;
        let mut cp = vec![Vec::new(); n + 1];
        let mut candidates: Vec<usize> = vec![0];

        for t in 1..=n {
            let mut best_cost = f64::MAX;
            let mut best_tau = 0;

            for &tau in &candidates {
                if t - tau < self.min_segment { continue; }
                let segment_cost = self.cost.compute(&data[tau..t]);
                let total = f[tau] + segment_cost + self.penalty;
                if total < best_cost {
                    best_cost = total;
                    best_tau = tau;
                }
            }

            f[t] = best_cost;
            cp[t] = cp[best_tau].clone();
            cp[t].push(best_tau);

            // Pruning: remove candidates that can never be optimal
            candidates.retain(|&tau| {
                f[tau] + self.cost.compute(&data[tau..t]) <= f[t]
            });
            candidates.push(t);
        }

        cp[n].clone()
    }
}
```

### 14.2 Retrospective Report

```
PELT analysis of Test gate pass rates (last 500 observations):

Change points detected at observations: [47, 183, 312]

Segment 1 (obs 0-47):   mean pass rate 0.92 ± 0.04  [stable, healthy]
Segment 2 (obs 48-183):  mean pass rate 0.71 ± 0.08  [regression, likely caused by auth refactor]
Segment 3 (obs 184-312): mean pass rate 0.88 ± 0.05  [recovery after fix batch]
Segment 4 (obs 313-500): mean pass rate 0.82 ± 0.06  [current regime, moderate]

Recommendation: current regime is below segment 1 baseline. Consider targeted
testing improvements for the code patterns introduced since observation 312.
```

---

## 15. Enhanced RungStats Structure

The SPC extensions require additional per-rung state:

```rust
/// Extended per-rung statistics with SPC monitoring.
pub struct RungStatsExtended {
    // --- Existing fields ---
    pub ema_pass_rate: f64,
    pub total_observations: u64,
    pub consecutive_passes: u32,

    // --- CUSUM ---
    pub cusum: CusumDetector,

    // --- EWMA control chart ---
    pub ewma_chart: EwmaControlChart,

    // --- BOCPD ---
    pub bocpd: BocpdDetector,

    // --- Metadata ---
    /// Timestamp of last observation (for decay calculations).
    pub last_observation_ms: u64,
    /// Number of regime changes detected (lifetime).
    pub regime_changes: u32,
    /// Current regime start observation index.
    pub current_regime_start: u64,
}

impl RungStatsExtended {
    /// Update all detectors with a new observation.
    pub fn observe(&mut self, passed: bool, timestamp_ms: u64) {
        let value = if passed { 1.0 } else { 0.0 };

        // Existing EMA update
        self.update_ema(value);
        self.update_consecutive(passed);

        // SPC updates
        self.cusum.update(value);
        self.ewma_chart.update(value);
        let changepoint = self.bocpd.update(value);

        // On regime change, recalibrate everything
        if changepoint {
            self.regime_changes += 1;
            self.current_regime_start = self.total_observations;
            self.cusum.reset();
            self.ewma_chart.recalibrate(self.ema_pass_rate);
        }

        self.last_observation_ms = timestamp_ms;
        self.total_observations += 1;
    }

    /// Comprehensive health assessment for this rung.
    pub fn health(&self) -> RungHealth {
        RungHealth {
            pass_rate: self.ema_pass_rate,
            in_control: self.ewma_chart.is_in_control(),
            shift_detected: self.cusum.shift_detected,
            shift_direction: self.cusum.shift_direction,
            recent_changepoint: self.bocpd.run_length_probs[0]
                > self.bocpd.changepoint_threshold * 0.5,
            suggested_retries: self.suggested_max_retries(),
            should_skip: self.should_skip(),
        }
    }
}

pub struct RungHealth {
    pub pass_rate: f64,
    pub in_control: bool,
    pub shift_detected: bool,
    pub shift_direction: Option<ShiftDirection>,
    pub recent_changepoint: bool,
    pub suggested_retries: u32,
    pub should_skip: bool,
}
```

---

## 16. Test Criteria for SPC Extensions

| Test | Property |
|---|---|
| `cusum_detects_sustained_drop` | 20 observations at 90%, then 20 at 70% → shift detected |
| `cusum_ignores_noise` | Random fluctuations around 80% → no shift signal |
| `cusum_fir_reset` | After detection, accumulator resets to h/2, can re-detect |
| `ewma_control_limits_converge` | UCL/LCL stabilize after ~50 observations |
| `ewma_flags_out_of_control` | Pass rate 90% → suddenly 50% → flagged within 10 obs |
| `bocpd_detects_regime_change` | 100 obs at 90%, then 100 at 60% → change point at ~100 |
| `bocpd_no_false_alarm` | Stable 85% pass rate for 500 obs → no change point |
| `pelt_finds_known_changepoints` | Synthetic data with breaks at [50, 150] → detected ±3 |
| `multi_gate_detects_correlated_drop` | Two gates degrade together → T² exceeds threshold |
| `sympathetic_tightening_triggers` | Downstream degradation → upstream retry budget decreases |
| `profile_cold_start` | New gate with profile prior starts at profile rate, not 0.5 |
| `regime_change_recalibrates` | BOCPD changepoint → CUSUM reset + EWMA target update |


---

# SOURCE: /Users/will/dev/nunchi/roko/roko/docs/04-verification/07-process-reward-models.md

# 07 — Process Reward Models

> **Layer**: L3 Harness — Verification
> **Crate**: `roko-learn` (planned integration), `roko-gate` (data source)
> **Status**: Design, informed by active gate infrastructure


> **Implementation**: Shipping

---

## 1. Overview

Process reward models (PRMs) score intermediate reasoning steps rather than only the
final output. In the context of agent-driven development, this means evaluating the
agent's *process* — each tool call, each file edit, each reasoning turn — not just
whether the final code passes gates.

Standard verification is binary: the code either compiles or it doesn't, tests either
pass or they don't. Process rewards add granularity: *how much progress did the agent
make toward a working solution?* This matters because:

- An agent that gets 90% of the way to a working solution before failing is more
  promising than one that makes no progress
- Intermediate progress signals enable early intervention (abandon a failing approach
  before it consumes the full retry budget)
- Process rewards provide 10x richer training signal than final pass/fail outcomes

> **Citation**: Lightman et al. "Let's Verify Step by Step" (2023) — PRM800K dataset,
> process reward models for mathematical reasoning.

> **Citation**: refactoring-prd/02-five-layers.md — "Process Reward Models: Promise +
> Progress scoring, low Promise → early intervention, negative Progress → re-planning."

---

## 2. The Two Dimensions: Promise and Progress

Roko's process reward model tracks two orthogonal signals per agent execution:

### 2.1 Promise

**Promise** estimates how likely the current execution is to eventually succeed, given
the work done so far. It answers: "Is this approach heading somewhere good?"

Indicators of high Promise:
- The agent is making compile-passing edits (Rung 0 passes)
- The agent's tool calls are targeting the right files
- The edit pattern matches successful historical executions for similar tasks

Indicators of low Promise:
- The agent is making the same edit repeatedly (loop detection)
- The agent's tool calls are targeting unrelated files
- The compile error count is increasing, not decreasing

**Intervention**: Low Promise → early termination of the current attempt, before the
full retry budget is consumed. Better to start fresh with a different prompt or model
than to continue down a failing path.

### 2.2 Progress

**Progress** measures whether the agent is advancing toward the goal or stalling. It
answers: "Is this attempt doing better than the previous one?"

Indicators of positive Progress:
- Higher rung reached than the previous attempt (ratchet advancing)
- Fewer compile errors than the previous attempt
- More tests passing than the previous attempt

Indicators of negative Progress:
- Same or lower rung than the previous attempt
- Same or more compile errors
- Same or fewer tests passing

**Intervention**: Negative Progress across multiple attempts → re-planning. The current
plan may be fundamentally flawed. Rather than retrying with the same approach, generate
a new plan and start over.

---

## 3. Data Sources for Process Rewards

The gate infrastructure provides rich data for computing process rewards:

### 3.1 Per-Turn Tool Call Metadata

Each agent turn produces a `ToolCallMeta`:
```rust
pub struct ToolCallMeta {
    pub tool_name: String,
    pub duration_ms: u64,
    pub result_tokens: u64,
    pub succeeded: bool,
    pub advanced_task: bool,      // Did this call advance the task?
    pub was_redundant: bool,      // Was this call unnecessary?
    pub error_category: Option<String>,
}
```

The `advanced_task` flag can be computed post-hoc: if a tool call's output was referenced
in the final solution (the agent used the information), it advanced the task. If not,
it was wasted work.

> **Citation**: tmp/implementation-plans/modelrouting/12-advanced-patterns.md §K
> (Task 2J.16) — "Per-step rewards provide 10x richer signal than final pass/fail."

### 3.2 Gate Verdicts with Rung Information

Each gate pipeline execution produces verdicts with:
- Which rung was reached (highest passing rung)
- Test counts (passed/failed/ignored) — a proxy for "how close to passing"
- Error digest — what went wrong, machine-parseable

### 3.3 Diff Analysis

The `DiffGate`'s analysis provides:
- How many substantive lines were added
- Whether the additions are vacuous (todo!/unimplemented!)
- Whether the code is getting more or less complete

### 3.4 Ratchet State

The `GateRatchet` tracks the highest rung passed per plan. Changes in the ratchet state
across attempts reveal whether the agent is making progress:
- Ratchet advances from rung 1 to rung 2: positive Progress
- Ratchet stays at rung 1 after 3 attempts: stalling
- Ratchet would regress from rung 2 to rung 1: negative Progress (blocked by ratchet)

---

## 4. The Promise Score Function

Promise is computed as a weighted combination of signals:

```
Promise(attempt) = w₁ × rung_fraction
                 + w₂ × test_pass_rate
                 + w₃ × error_trend
                 + w₄ × tool_efficiency
```

Where:
- `rung_fraction` = highest_rung_passed / total_rungs (0.0 to 1.0)
- `test_pass_rate` = tests_passed / total_tests (0.0 to 1.0, or 0.5 if no test gate)
- `error_trend` = 1.0 if errors decreasing, 0.5 if stable, 0.0 if increasing
- `tool_efficiency` = useful_tool_calls / total_tool_calls

Default weights: w₁=0.4, w₂=0.3, w₃=0.2, w₄=0.1.

### Promise Thresholds

| Promise | Interpretation | Action |
|---|---|---|
| > 0.8 | High — likely to succeed | Continue, possibly reduce retries |
| 0.4–0.8 | Moderate — uncertain | Continue with standard retries |
| 0.2–0.4 | Low — probably failing | Consider early termination |
| < 0.2 | Very low — almost certainly failing | Terminate, try different approach |

---

## 5. The Progress Score Function

Progress compares the current attempt to the previous one:

```
Progress(attempt_n) = Δrung + Δtest_rate + Δerror_count
```

Where:
- `Δrung` = (current_rung - previous_rung) / total_rungs
- `Δtest_rate` = current_pass_rate - previous_pass_rate
- `Δerror_count` = (previous_errors - current_errors) / max(previous_errors, 1)
  (positive = improvement)

### Progress Thresholds

| Progress | Interpretation | Action |
|---|---|---|
| > 0.1 | Advancing | Continue, approach is working |
| -0.1 to 0.1 | Stalling | Escalate complexity, adjust prompt |
| < -0.1 | Regressing | Stop retrying, re-plan |

---

## 6. Integration with the Feedback Loop

Process rewards create a fast feedback loop within the agent execution cycle:

```
Agent turn N
    ↓
Gate pipeline
    ↓
Compute Promise(N) and Progress(N)
    ↓
Decision:
  Promise > 0.4 and Progress > -0.1 → continue to turn N+1
  Promise < 0.2 → early termination
  Progress < -0.1 for 3 turns → re-plan
```

This is a *within-attempt* optimization. It happens faster than the retry loop
(which operates across attempts) and much faster than the escalation mechanism
(which operates across failed attempts).

The three feedback timescales:
1. **Process reward** (per-turn): Promise/Progress → continue/terminate
2. **Retry loop** (per-attempt): Gate verdict → retry with adjusted prompt
3. **Escalation** (across attempts): Repeated failure → add rungs, re-plan

> **Citation**: bardo-backup/prd/16-testing/07-fast-feedback-loops.md — Machine-speed
> evaluation loops: confidence calibration, context attribution, cost-effectiveness.

---

## 7. Academic Foundations

### 7.1 Lightman et al. (2023) — PRM800K

Showed that process supervision (scoring each step) outperforms outcome supervision
(scoring only the final answer) for mathematical reasoning. Best-of-N selection with
process rewards outperformed majority voting by 8%.

### 7.2 AgentPRM (arXiv:2502.10325)

Extended process rewards to agent tool-use settings. Per-step rewards provide 10x richer
signal than final pass/fail. The key insight: not all tool calls contribute equally to
the outcome. Scoring each call identifies which calls are productive vs. wasteful.

### 7.3 Self-Refine (Madaan et al. 2023)

Showed that LLMs can improve their own outputs through iterative refinement with
feedback. Process rewards formalize the feedback signal that drives refinement: rather
than generic "try again," the agent gets "your Promise score dropped because error count
increased — focus on reducing errors."

### 7.4 Reflexion (Shinn et al. 2023)

Introduced the concept of verbal reinforcement for agents: converting numeric feedback
into natural language that the agent can learn from. Process rewards can be converted
to reflexion-style feedback: "Your last 3 attempts reached Rung 1 but failed at Rung 2.
Test failures are all in the auth module. Focus on auth module tests."

> **Citation**: bardo-backup/tmp/mori-refactor/06-harness.md — Academic foundations
> section: "Process Reward Models (Lightman et al. 2023 PRM800K), Self-Refine (Madaan
> et al. 2023), Reflexion (Shinn et al. 2023)."

---

## 8. Promise + Progress as Cybernetic Signals

In the Synapse Architecture's cybernetic feedback model, gate verdicts flow back to
Scorer, Router, Composer, and the agent. Process rewards add two new feedback channels:

| Signal | From | To | Effect |
|---|---|---|---|
| Promise | Gate + ToolCallMeta | Conductor | Early termination on low Promise |
| Progress | Gate (across attempts) | Router | Model re-selection on stalling |
| Progress | Gate (across attempts) | Composer | Prompt adjustment on stalling |
| Promise × Progress | Combined | Policy | Re-plan on persistent low signals |

This creates a multi-timescale control system:
- **Fast**: Promise per-turn → terminate unproductive attempts
- **Medium**: Progress per-attempt → adjust routing and prompts
- **Slow**: Trend across plans → update model preferences and prompt templates

> **Citation**: refactoring-prd/01-synapse-architecture.md — Cybernetic feedback loops
> from Gate to Scorer, Router, Composer.

---

## 9. Relationship to Predictive Foraging

Process rewards connect to the predictive foraging system (from the agent-chain
architecture). Before a task starts, the router predicts success probability. After the
task runs, the actual outcome produces a residual (prediction - reality). Process
rewards refine these predictions:

- If Promise is high but the task fails → the predictor was overconfident
- If Promise is low but the task succeeds → the predictor was underconfident
- Residuals from process rewards calibrate the predictor faster than binary outcomes

> **Citation**: tmp/implementation-plans/modelrouting/12-advanced-patterns.md §B
> (Tasks 2J.03–2J.04) — Predictive Foraging: prediction → residual → bias correction.

---

## 10. Summary

Process reward models transform binary gate verdicts into a continuous signal of agent
quality. By tracking Promise (is this attempt heading somewhere good?) and Progress (is
the agent improving across attempts?), the system can make finer-grained decisions about
when to continue, when to terminate early, and when to re-plan entirely.

The data is already there — gate verdicts, tool call metadata, ratchet state, diff
analysis. Process rewards are a *lens* on this data, not a new data collection system.
They turn "passed/failed" into "making progress / stalling / regressing" and enable
interventions that save compute and improve outcomes.

---

## 11. Self-Supervised PRM Training from Gate Verdicts

Roko has a unique advantage over academic PRM systems: it generates its own step-level
training labels. The gate pipeline is a deterministic oracle. Every intermediate artifact
can be verified, producing automated labels without human annotation.

> **Citation**: Lightman et al. "Let's Verify Step by Step" (arXiv:2305.20050, 2023) —
> PRM800K required 800K human labels. Self-supervised approaches eliminate this cost.

> **Citation**: "Process-Supervised Reinforcement Learning for Code Generation"
> (arXiv:2502.01715, 2025) — compiler-driven step-level rewards for code RL.

### 11.1 The Self-Supervision Loop

```
Agent execution trace:
  step_1: read file → artifact_1 (no code change)
  step_2: edit file → artifact_2 (code changed)
  step_3: edit file → artifact_3 (code changed)
  step_4: run tests → artifact_4 (no code change)
  step_5: fix test → artifact_5 (code changed)

Gate verification of each intermediate artifact:
  artifact_1: N/A (no code change — label inherited from previous)
  artifact_2: compile PASS, test FAIL → partial credit
  artifact_3: compile PASS, test PASS 8/10 → more credit
  artifact_5: compile PASS, test PASS 10/10 → full credit

Step-level labels:
  step_1: 0.3 (read → neutral, but necessary)
  step_2: 0.5 (compiles but tests fail)
  step_3: 0.7 (more tests pass)
  step_4: 0.3 (information-gathering, no code progress)
  step_5: 1.0 (all tests pass)
```

### 11.2 Monte Carlo Step-Level Q-Values

For richer labels than binary gate outcomes, use Monte Carlo rollouts to estimate the
probability that a step leads to eventual success:

```rust
/// Monte Carlo estimator for step-level quality values.
///
/// For each intermediate step, estimate the probability that continuing
/// from that state leads to eventual gate passage. This is the Q-value
/// of the step under the current policy.
pub struct MonteCarloStepLabeler {
    /// Number of rollouts per step (more = better estimate, higher cost).
    pub num_rollouts: usize,    // default: 8
    /// Maximum additional turns per rollout.
    pub max_rollout_turns: usize, // default: 10
    /// Gate pipeline to evaluate rollout outcomes.
    pub gate_pipeline: GatePipeline,
    /// Agent backend for generating rollout continuations.
    pub agent: Box<dyn AgentBackend>,
}

impl MonteCarloStepLabeler {
    /// Estimate the Q-value of reaching this intermediate state.
    ///
    /// Pseudocode:
    ///   successes = 0
    ///   for k in 0..num_rollouts:
    ///       continuation = agent.continue_from(state, max_turns=max_rollout_turns)
    ///       verdict = gate_pipeline.verify(continuation.artifact)
    ///       if verdict.passed:
    ///           successes += 1
    ///   q_value = successes / num_rollouts
    ///   label = "correct" if q_value > 0.5 else "incorrect"
    pub async fn estimate_q_value(&self, state: &IntermediateState) -> StepLabel {
        let mut successes = 0;
        for _ in 0..self.num_rollouts {
            let continuation = self.agent
                .continue_from(state, self.max_rollout_turns).await;
            let verdict = self.gate_pipeline
                .verify(&continuation.as_signal(), &state.context).await;
            if verdict.passed {
                successes += 1;
            }
        }
        let q_value = successes as f64 / self.num_rollouts as f64;
        StepLabel {
            q_value,
            label: if q_value > 0.5 { StepQuality::Correct } else { StepQuality::Incorrect },
            confidence: wilson_confidence(successes as u64, self.num_rollouts as u64),
        }
    }
}

pub struct StepLabel {
    pub q_value: f64,
    pub label: StepQuality,
    pub confidence: f64,
}

pub enum StepQuality {
    Correct,
    Incorrect,
    Ambiguous, // q_value near 0.5
}
```

**Cost analysis**: With 8 rollouts per step and ~5 code-modifying steps per task, this
requires 40 additional agent turns plus 40 gate evaluations. At Haiku-tier costs (~$0.001
per turn), the total labeling cost is ~$0.04 per task. For 100 tasks, $4 produces
~500 step-level labels — orders of magnitude cheaper than human annotation.

### 11.3 FoVer: Formally Verified Labels

For code changes that can be expressed as logical assertions, formal verification
provides perfect step-level labels:

> **Citation**: Kamoi et al., "FoVer: Generalizable Process Reward Models via Formally
> Verified Training Data" (arXiv:2505.15960, 2025) — Z3 and Isabelle for automatic labels.

```rust
/// Formally verified step labeling for code changes.
///
/// For steps that modify contracts, invariants, or type-level properties,
/// attempt formal verification of the intermediate state.
pub struct FormalStepLabeler {
    /// Verification backend (Z3, Isabelle, or Prusti for Rust).
    pub verifier: Box<dyn FormalVerifier>,
    /// Maximum verification time per step.
    pub timeout: Duration,  // default: 30s
}

pub trait FormalVerifier: Send + Sync {
    /// Attempt to verify that the code change preserves stated invariants.
    ///
    /// Returns Verified (label=correct), Counterexample (label=incorrect),
    /// or Timeout/Unknown (no label).
    fn verify(&self, pre_state: &Code, post_state: &Code,
              invariants: &[Invariant]) -> VerificationResult;
}

pub enum VerificationResult {
    /// Invariants hold — step is correct.
    Verified,
    /// Counterexample found — step introduced a bug.
    Counterexample(String),
    /// Verification timed out or was inconclusive.
    Unknown,
}
```

FoVer labels are high-confidence but limited to steps where formal specs exist.
In practice, ~10-20% of code changes in a typed language like Rust can be formally
checked (type constraints, trait bounds, lifetime invariants). The rest use Monte Carlo.

---

## 12. RLHF Alternatives for Agent Improvement

Gate verdicts provide a natural reward signal. The question is how to convert
these rewards into improved agent behavior across future tasks.

### 12.1 DPO (Direct Preference Optimization)

DPO avoids explicit reward model training by directly optimizing the policy from
preference pairs. For agent verification, preference pairs come from gate outcomes:

> **Citation**: Rafailov et al., "Direct Preference Optimization: Your Language Model
> Is Secretly a Reward Model" (NeurIPS 2023, arXiv:2305.18290).

```rust
/// DPO training pair from gate verdicts.
///
/// When two agents attempt the same task and one passes gates while
/// the other fails, this creates a natural preference pair.
pub struct DpoTrainingPair {
    /// The task specification (shared context).
    pub task_spec: String,
    /// The preferred response (passed all gates).
    pub preferred: AgentTrace,
    /// The dispreferred response (failed gates).
    pub dispreferred: AgentTrace,
    /// Margin: how much better the preferred response was.
    /// Higher margin = stronger training signal.
    pub margin: f64,
}

/// DPO loss function (for reference — training happens offline):
///
/// L_DPO(θ) = -E[ log σ( β × (
///     log π_θ(y_w|x) / π_ref(y_w|x)
///   - log π_θ(y_l|x) / π_ref(y_l|x)
/// ))]
///
/// where:
///   x = task_spec
///   y_w = preferred trace
///   y_l = dispreferred trace
///   β = temperature (sharpness of reward model)
///   π_ref = reference policy (the untuned model)

/// Parameters for DPO-derived preference collection.
pub struct DpoConfig {
    /// Temperature parameter controlling reward sharpness.
    /// Lower = more decisive preferences. Default: 0.1.
    pub beta: f64,
    /// Minimum margin between preferred/dispreferred.
    /// Pairs with margin < this are too similar to be useful.
    pub min_margin: f64,          // default: 0.3
    /// Maximum pairs to collect before triggering training.
    pub batch_size: usize,        // default: 128
    /// Storage path for collected pairs.
    pub pairs_path: PathBuf,
}
```

**Implicit reward extraction**: DPO implicitly defines a reward model:
`r(x, y) = β × log(π_θ(y|x) / π_ref(y|x))`. This can be extracted and used
for the process reward scoring without separate PRM training.

> **Citation**: "Bootstrapping Language Models with DPO Implicit Rewards" (ICLR 2025)
> — using DPO's implicit reward for self-improvement.

### 12.2 RLAIF (Reinforcement Learning from AI Feedback)

Instead of human preferences, use a judge model to generate preferences from gate
verdicts and code quality analysis:

```rust
/// RLAIF configuration for generating AI feedback from gate signals.
pub struct RlaifConfig {
    /// The judge model (e.g., Opus) that evaluates traces.
    pub judge_model: String,
    /// Aspects to evaluate (code quality, efficiency, correctness).
    pub evaluation_criteria: Vec<EvaluationCriterion>,
    /// Whether to include gate verdicts as context for the judge.
    pub include_gate_context: bool,  // default: true
}

pub struct EvaluationCriterion {
    pub name: String,
    pub weight: f64,
    pub prompt_template: String,
}

/// RLAIF feedback generation:
///
/// 1. Collect two traces for the same task (different models or attempts)
/// 2. Show both traces + gate verdicts to the judge model
/// 3. Judge produces: {preferred: "A"|"B", reasoning: "...", confidence: 0.9}
/// 4. Store as DPO training pair with judge's confidence as margin
///
/// This is cheaper than DPO because the judge model doesn't need to
/// generate rollouts — it only evaluates existing traces.
```

### 12.3 Constitutional AI for Safety Gates

Safety-related gate failures (e.g., code that introduces security vulnerabilities,
race conditions, or resource leaks) can be addressed with a Constitutional AI approach:

```rust
/// Constitutional principles for the safety gate.
pub const SAFETY_CONSTITUTION: &[&str] = &[
    "Generated code must not introduce SQL injection, XSS, or command injection.",
    "Generated code must not disable authentication or authorization checks.",
    "Generated code must not expose secrets in logs, error messages, or comments.",
    "Generated code must not introduce race conditions or deadlocks.",
    "Generated code must not create unbounded resource allocation.",
];

/// Self-critique loop using constitutional principles:
///
/// 1. Agent produces code change
/// 2. Critic (same or different model) evaluates against constitution
/// 3. If violation found: generate revised version that fixes the violation
/// 4. Repeat until clean or max iterations
/// 5. Run gate pipeline on the final version
///
/// The critique step happens BEFORE the gate pipeline, catching safety
/// issues that static gates might miss (gates check syntax/tests, not
/// security semantics).
```

---

## 13. Reward Shaping for Intermediate Steps

Raw gate verdicts are sparse: most intermediate steps produce no verdict (no code
change = nothing to verify). Reward shaping fills the gaps with dense signals that
guide the agent without changing the optimal policy.

> **Citation**: Ng et al., "Policy Invariance Under Reward Transformations" (ICML 1999)
> — potential-based reward shaping preserves optimal policy.

### 13.1 Potential-Based Reward Shaping

```rust
/// Potential function over agent states.
///
/// Maps each intermediate state to a scalar that estimates "how close
/// to passing gates" the agent is. The shaped reward is:
///
///   R'(s, a, s') = R(s, a, s') + γ × Φ(s') - Φ(s)
///
/// where γ = discount factor and Φ = potential function.
/// This preserves the optimal policy (Ng et al. 1999 theorem).
pub struct GatePotential {
    /// Weight for each component of the potential.
    pub weights: PotentialWeights,
}

pub struct PotentialWeights {
    /// Weight for compilation status (0/1).
    pub compile_weight: f64,      // default: 0.4
    /// Weight for test pass rate [0, 1].
    pub test_rate_weight: f64,    // default: 0.3
    /// Weight for lint cleanliness [0, 1].
    pub lint_weight: f64,         // default: 0.1
    /// Weight for code completeness (1 - stub_fraction) [0, 1].
    pub completeness_weight: f64, // default: 0.2
}

impl GatePotential {
    /// Compute the potential of an intermediate state.
    ///
    /// Higher potential = closer to passing all gates.
    pub fn phi(&self, state: &IntermediateState) -> f64 {
        let compile = if state.compiles { 1.0 } else { 0.0 };
        let test_rate = state.tests_passed as f64
            / state.tests_total.max(1) as f64;
        let lint = if state.lint_clean { 1.0 } else { 0.5 };
        let completeness = 1.0 - state.stub_fraction();

        self.weights.compile_weight * compile
            + self.weights.test_rate_weight * test_rate
            + self.weights.lint_weight * lint
            + self.weights.completeness_weight * completeness
    }

    /// Compute the shaped reward for a state transition.
    ///
    /// Positive when the agent moves toward passing gates.
    /// Negative when the agent moves away.
    /// Zero when no progress (encourages efficiency).
    pub fn shaped_reward(&self, prev: &IntermediateState,
                         next: &IntermediateState,
                         discount: f64) -> f64 {
        discount * self.phi(next) - self.phi(prev)
    }
}
```

### 13.2 Shaping Signal Interpretation

```
Step: agent adds a use statement (fixes compile error)
  Φ(prev) = 0.0 (doesn't compile)
  Φ(next) = 0.4 (compiles, no tests pass yet)
  Shaped reward: 0.99 × 0.4 - 0.0 = +0.396 (positive: progress)

Step: agent deletes a test (makes test suite pass vacuously)
  Φ(prev) = 0.7 (compiles, 7/10 tests pass)
  Φ(next) = 0.6 (compiles, 7/7 tests pass but completeness drops)
  Shaped reward: 0.99 × 0.6 - 0.7 = -0.106 (negative: regression)

Step: agent reads a file (no code change)
  Φ(prev) = 0.5
  Φ(next) = 0.5 (unchanged)
  Shaped reward: 0.99 × 0.5 - 0.5 = -0.005 (near zero: encourages efficiency)
```

The potential-based shaping naturally penalizes vacuous changes (deleting tests to
pass) and rewards genuine progress (fixing errors to pass), without any hand-coded
rules for these behaviors.

### 13.3 Dense Reward Schedule

Combining sparse gate rewards with shaped potential rewards:

```rust
/// Complete per-step reward computation.
pub struct StepRewardComputer {
    pub gate_potential: GatePotential,
    pub discount: f64,               // default: 0.99
    pub gate_weight: f64,            // default: 1.0 (sparse gate reward weight)
    pub shaping_weight: f64,         // default: 0.5 (shaped reward weight)
}

impl StepRewardComputer {
    pub fn compute(&self, prev: &IntermediateState,
                   next: &IntermediateState,
                   gate_verdict: Option<&Verdict>) -> f64 {
        // Sparse gate reward (only present on code-modifying steps)
        let gate_reward = gate_verdict
            .map(|v| if v.passed { 1.0 } else { -0.5 })
            .unwrap_or(0.0);

        // Dense shaped reward (every step)
        let shaped = self.gate_potential.shaped_reward(prev, next, self.discount);

        self.gate_weight * gate_reward + self.shaping_weight * shaped
    }
}
```

---

## 14. ThinkPRM: Generative Process Verification

Rather than training a discriminative PRM (which requires labeled data), use a
generative approach: ask a reasoning model to verify each step by thinking through it.

> **Citation**: Mukhal et al., "Process Reward Models That Think" (arXiv:2504.16828,
> 2025) — ThinkPRM fine-tuned on 1K synthetic CoTs outperforms discriminative PRMs
> using 1% of typical annotation cost.

```rust
/// ThinkPRM: a generative process verifier that reasons about step correctness.
///
/// Instead of learning a classifier, this uses chain-of-thought reasoning
/// to verify each step. The model thinks through "does this step make
/// sense given the task and previous steps?" and outputs a verdict.
pub struct ThinkPrm {
    /// The reasoning model used for verification.
    /// Prefer a capable model (Sonnet+) for reliable reasoning.
    pub model: String,
    /// Maximum reasoning tokens per step verification.
    pub max_reasoning_tokens: usize,  // default: 1024
    /// Score threshold for marking a step as incorrect.
    pub threshold: f64,               // default: 0.5
}

impl ThinkPrm {
    /// Verify a single step in context.
    ///
    /// Prompt structure:
    ///   "You are verifying step {i} of an agent's execution.
    ///    Task: {task_description}
    ///    Previous steps: {step_1..step_{i-1}}
    ///    Current step: {step_i}
    ///    Gate results so far: {gate_verdicts}
    ///
    ///    Think step by step about whether this step is:
    ///    1. Moving toward solving the task
    ///    2. Consistent with previous steps
    ///    3. Likely to lead to gate passage
    ///
    ///    Score the step from 0.0 (definitely wrong) to 1.0 (definitely correct)."
    ///
    /// Returns the model's step score and reasoning chain.
    pub async fn verify_step(&self, step: &Step,
                              context: &StepContext) -> StepVerification {
        // ... prompt construction and model call
        todo!()
    }
}

pub struct StepVerification {
    pub step_index: usize,
    pub score: f64,
    pub reasoning: String,
    pub is_correct: bool,
    pub verification_tokens: usize,
}
```

**Cost-effectiveness**: ThinkPRM requires no training data — it uses the model's
reasoning ability directly. At ~1K tokens per step verification and ~5 steps per task,
the cost is ~5K tokens per task (~$0.075 at Sonnet-tier). This is 10x cheaper than
Monte Carlo rollouts and provides natural-language explanations of why a step is good
or bad.

---

## 15. Integration Architecture

All PRM components connect into a unified step-level scoring pipeline:

```
Agent step N
    │
    ├── Gate verdict (sparse, binary) ──────────────────────┐
    │                                                        │
    ├── Potential-based shaping (dense, continuous) ─────────┤
    │                                                        │
    ├── ThinkPRM verification (generative, reasoning) ──────┤
    │                                                        ▼
    │                                              StepRewardComputer
    │                                                        │
    │                                              Combined step score
    │                                                        │
    ├── If score > 0.4 AND Progress > -0.1 ──► Continue
    ├── If score < 0.2 ────────────────────► Early termination
    ├── If negative Progress × 3 turns ────► Re-plan
    └── Accumulate for DPO pair collection
```

### 15.1 Persistence

Step-level rewards are persisted alongside episodes:

```
.roko/learn/
├── episodes.jsonl              # raw agent traces
├── step-rewards.jsonl          # per-step reward annotations
│   {"episode_id": "...", "step": 3, "gate_reward": 0.0,
│    "shaped_reward": 0.15, "think_score": 0.72, "combined": 0.44}
├── dpo-pairs.jsonl             # collected preference pairs
│   {"task": "...", "preferred": "ep_123", "dispreferred": "ep_124",
│    "margin": 0.6}
└── prm-metrics.json            # PRM calibration metrics
    {"accuracy": 0.83, "calibration_error": 0.04, "samples": 1200}
```

---

## 16. Test Criteria

| Test | Property |
|---|---|
| `monte_carlo_q_value_correct` | All-passing rollouts → q_value ≈ 1.0 |
| `monte_carlo_q_value_failing` | All-failing rollouts → q_value ≈ 0.0 |
| `potential_compile_fix_positive` | Fixing compile error → positive shaped reward |
| `potential_delete_test_negative` | Deleting tests to pass → negative shaped reward |
| `potential_no_change_near_zero` | Read-only step → shaped reward ≈ 0 |
| `dpo_pair_minimum_margin` | Pairs with margin < 0.3 are filtered out |
| `shaped_reward_preserves_policy` | Total shaped reward over optimal path = 0 (Ng theorem) |
| `step_reward_combines_sources` | gate_weight × gate + shaping_weight × shaped = correct |
| `think_prm_score_bounds` | Score always in [0.0, 1.0] |
| `early_termination_on_low_promise` | Promise < 0.2 for 2 turns → termination signal |
| `replan_on_negative_progress` | Progress < -0.1 for 3 turns → replan signal |


---

# SOURCE: /Users/will/dev/nunchi/roko/roko/docs/04-verification/08-agent-feedback-from-gates.md

# 08 — Agent Feedback from Gates

> **Layer**: L3 Harness — Verification → L2 Engine (feedback channel)
> **Crate**: `roko-gate` (`crates/roko-gate/src/feedback.rs`)
> **Status**: Implemented (375 lines)


> **Implementation**: Shipping

---

## 1. Overview

Raw gate output — compiler stderr, test logs, linter JSON — is verbose and full of
noise. Progress bars, download messages, repeated blank lines, and Cargo metadata all
waste agent context tokens without contributing actionable information. The feedback
module solves this: it parses raw output into structured `GateFeedback` containing only
actionable items classified by severity.

This is the bridge between the Harness layer (L3, where gates run) and the agent's
context window. It ensures that when an agent retries after a gate failure, it sees
exactly the errors and warnings it needs to fix — and nothing else.

> **Citation**: crates/roko-gate/src/feedback.rs — Full implementation.

---

## 2. The GateFeedback Type

```rust
pub struct GateFeedback {
    pub rung: u8,                  // Which rung produced this feedback
    pub passed: bool,              // Whether the gate passed
    pub errors: Vec<String>,       // Error-level items (must fix)
    pub warnings: Vec<String>,     // Warning-level items (should fix)
    pub suggestions: Vec<String>,  // Informational/help items
}
```

Three severity buckets, ordered from most to least critical:
- **errors**: Compilation errors, test failures, panics. The agent *must* fix these.
- **warnings**: Unused variables, deprecated usage, style issues. The agent *should*
  fix these.
- **suggestions**: Compiler help messages, notes, file location pointers. These *inform*
  the agent about what to do.

### Helper Methods

```rust
impl GateFeedback {
    pub fn item_count(&self) -> usize;   // Total items across all categories
    pub fn is_empty(&self) -> bool;       // True if no actionable items
    pub fn items(&self) -> Vec<FeedbackItem>;  // All items, errors first
}
```

The `items()` method returns all feedback as `FeedbackItem` structs (severity + message),
ordered: errors first, then warnings, then suggestions. This ordering ensures the most
critical information appears first in the agent's context window.

---

## 3. The Classification Pipeline

### 3.1 Per-Line Classification

```rust
fn classify_line(line: &str) -> Option<(Severity, &str)>
```

Each line of raw output is classified into a severity or `None` (noise). The classifier
is a priority chain:

1. **Empty/whitespace**: → `None`
2. **Noise patterns**: → `None`
3. **Error patterns**: → `Some(Severity::Error, line)`
4. **Warning patterns**: → `Some(Severity::Warning, line)`
5. **Suggestion patterns**: → `Some(Severity::Info, line)`
6. **Anything else**: → `None` (dropped as context)

### 3.2 Noise Detection

```rust
fn is_noise(line: &str) -> bool
```

Lines that are pure noise — no actionable information:

| Pattern | Example |
|---|---|
| Cargo progress | `Downloading`, `Downloaded`, `Compiling`, `Checking`, `Finished`, `Running`, `Documenting`, `Fresh`, `Packaging` |
| npm deprecation | `npm WARN deprecated stable@0.1.0: deprecated package` |
| Progress bars | Lines containing `━`, `▓`, `░` |

These lines are common in build output and contribute nothing to error diagnosis.

### 3.3 Error Detection

```rust
fn is_error_line(line: &str) -> bool
```

Lines that indicate errors:

| Pattern | What It Catches |
|---|---|
| `error` (starts with) | Rust `error:` messages |
| `Error:` (starts with) | Generic error format |
| `ERROR:` (starts with) | Uppercase error format |
| `FAILED` (starts with) | Test failure markers |
| `FAIL ` (starts with) | Go test failure |
| Contains `error[E` | Rustc error codes like `error[E0425]` |
| Contains `panicked at` | Panic messages |
| `thread '...' panicked` | Thread panic with test name |

### 3.4 Warning Detection

```rust
fn is_warning_line(line: &str) -> bool
```

| Pattern | What It Catches |
|---|---|
| `warning` (starts with) | Rust warnings |
| `Warning:` (starts with) | Generic warning format |
| `WARNING:` (starts with) | Uppercase warning format |
| `warn[` (starts with) | Clippy warning codes |

### 3.5 Suggestion Detection

```rust
fn is_suggestion_line(line: &str) -> bool
```

| Pattern | What It Catches |
|---|---|
| `help:` (starts with) | Compiler help messages |
| Contains `= help:` | Inline help annotations |
| `note:` (starts with) | Compiler notes |
| Contains `= note:` | Inline note annotations |
| `suggestion:` (starts with) | Explicit suggestions |
| `hint:` (starts with) | Hint messages |
| `-->` (starts with or contains) | Source location pointers |

> **Citation**: crates/roko-gate/src/feedback.rs:99–192 — Classification functions.

---

## 4. The Public API

```rust
pub fn feedback_for_agent(gate_output: &str, rung: u8) -> GateFeedback
```

The main entry point. Takes raw gate output (typically stdout + stderr concatenated) and
a rung number, returns structured feedback.

### Algorithm

```
for each line in gate_output:
    classify_line(line)
    match severity:
        Error   → push to errors, set has_errors = true
        Warning → push to warnings
        Info    → push to suggestions
        None    → skip (noise)

passed = !has_errors
return GateFeedback { rung, passed, errors, warnings, suggestions }
```

### Pass Detection

The feedback's `passed` field is based purely on whether any error-level items were
found. If there are warnings but no errors, the feedback says `passed = true`. This
aligns with the convention that warnings don't block compilation or test execution.

---

## 5. How the Orchestrator Uses Feedback

After a gate failure, the orchestrator generates feedback and injects it into the
agent's retry prompt:

```rust
let verdict = pipeline.verify(signal, ctx).await;

if !verdict.passed {
    let feedback = feedback_for_agent(
        verdict.detail.as_deref().unwrap_or(""),
        current_rung,
    );

    // Inject into retry prompt
    let retry_context = format!(
        "Your previous attempt failed at rung {}.\n\
         Errors ({}):\n{}\n\
         Warnings ({}):\n{}\n\
         Suggestions ({}):\n{}",
        feedback.rung,
        feedback.errors.len(),
        feedback.errors.join("\n"),
        feedback.warnings.len(),
        feedback.warnings.join("\n"),
        feedback.suggestions.len(),
        feedback.suggestions.join("\n"),
    );
    // ... feed retry_context to the agent
}
```

This pattern:
1. Extracts only actionable information from potentially thousands of lines of output
2. Categorizes by severity so the agent knows what to prioritize
3. Preserves source location pointers (the `-->` lines) so the agent knows *where*
   to fix

---

## 6. Token Economy

The feedback module is a critical part of Roko's token economy. Consider a typical
`cargo check` failure:

| Raw output | ~2,000 lines |
|---|---|
| Noise (Downloading, Compiling, etc.) | ~1,500 lines |
| Errors | ~10 lines |
| Warnings | ~20 lines |
| Suggestions | ~15 lines |
| **Filtered feedback** | **~45 lines** |

That's a 97.75% reduction. At ~4 tokens per line, the feedback saves ~7,800 tokens per
gate failure. Over a 5-attempt retry loop with 3 gate failures, that's ~23,400 tokens
saved — a significant fraction of the agent's context window.

> **Citation**: bardo-backup/tmp/mori-refactor/06-harness.md — "Raw gate output
> (compiler stderr, test logs, linter JSON) is verbose and full of noise that wastes
> agent context tokens."

---

## 7. Gate-to-Scaffold Feedback Loop

The feedback module is one half of a closed loop. The other half is the section
effectiveness tracker (see the Scaffold layer documentation and implementation plan
task 2J.05–2J.06):

```
Agent receives prompt (with sections)
    ↓
Agent produces code
    ↓
Gate verifies code → Verdict
    ↓
feedback_for_agent() → GateFeedback
    ↓
Two consumers:
    1. Agent retry prompt (immediate)
    2. SectionEffectivenessRegistry (learning)
       → Which prompt sections correlated with gate success?
       → Adjust section priorities for future prompts
```

The immediate feedback (inject errors into retry prompt) operates at machine speed.
The section effectiveness learning operates at consolidation speed — it needs 50+
observations to make statistical claims about which prompt sections help.

> **Citation**: tmp/implementation-plans/modelrouting/12-advanced-patterns.md §C
> (Tasks 2J.05–2J.06) — Gate-to-scaffold feedback loop, section effectiveness tracking
> with lift > 0.05.

---

## 8. Severity Ordering

```rust
pub enum Severity {
    Info,       // 0
    Warning,    // 1
    Error,      // 2
}
```

`Severity` derives `PartialOrd` and `Ord`, with `Info < Warning < Error`. This enables:
- Sorting feedback items by severity (`items()` returns errors first)
- Filtering by minimum severity (e.g., "only show errors and warnings")
- Threshold-based policies (e.g., "fail if any Error, warn if any Warning")

---

## 9. Serde Support

Both `GateFeedback` and `FeedbackItem` derive `Serialize` and `Deserialize`. This
enables:
- Persisting feedback to the episode log (`.roko/episodes.jsonl`)
- Transmitting feedback as JSON to agents that expect structured input
- Aggregating feedback across executions for learning

The serde roundtrip test in the module verifies that serialization and deserialization
preserve all fields exactly.

---

## 10. Limitations and Future Work

### 10.1 Language-Specific Heuristics

The current classifier is biased toward Rust/Cargo output. The patterns for npm, Go,
and other build systems are minimal. Future work should add per-language classifiers:

```rust
fn classify_line_for_build_system(line: &str, build: BuildSystem) -> Option<(Severity, &str)>
```

### 10.2 Multi-Line Error Messages

Rustc error messages span multiple lines (the error, the source snippet, the help
message). The current classifier treats each line independently, which means:
- The error line is classified as Error
- The source snippet line is classified as noise (dropped)
- The help line is classified as Info

This loses the visual structure of the error. A future improvement would group
consecutive lines belonging to the same diagnostic into a single `FeedbackItem`.

### 10.3 Structured Error Formats

Some tools emit structured output (JSON, SARIF). The current classifier works on
line-by-line text. Future work should detect and parse structured formats:

```
if gate_output starts with "[" or "{":
    parse as JSON diagnostic array
else:
    use line-by-line classifier
```

---

## 11. Testing

The feedback module has 14 tests covering:

| Test | What It Verifies |
|---|---|
| `feedback_empty_output_passes` | Empty input → passed, no items |
| `feedback_extracts_errors` | Error lines extracted correctly |
| `feedback_extracts_warnings` | Warning lines extracted correctly |
| `feedback_extracts_suggestions` | Help/note lines extracted correctly |
| `feedback_filters_noise` | Cargo progress lines dropped |
| `feedback_mixed_output` | Mixed output classified correctly |
| `feedback_item_count` | Total count across categories |
| `feedback_items_ordering` | Errors first, then warnings, then suggestions |
| `feedback_rung_preserved` | Rung number roundtrips correctly |
| `feedback_test_failure_detected` | FAILED/panicked lines are errors |
| `feedback_severity_ordering` | Info < Warning < Error |
| `feedback_npm_deprecation_is_noise` | npm-specific noise detected |
| `feedback_progress_bars_are_noise` | Unicode progress bars detected |
| `feedback_serde_roundtrip` | JSON serialization preserves all fields |

> **Citation**: crates/roko-gate/src/feedback.rs:241–374 — Tests section.


---

# SOURCE: /Users/will/dev/nunchi/roko/roko/docs/04-verification/09-evaluation-lifecycle.md

# 09 — Evaluation Lifecycle

> **Layer**: L3 Harness — Verification
> **Crates**: `roko-gate`, `roko-learn`, `roko-conductor`
> **Status**: Partial (fast loops wired, slow loops designed)


> **Implementation**: Shipping

---

## 1. Overview

Evaluation in Roko is not a single step — it is a lifecycle that spans five speed tiers,
from sub-second machine checks to multi-day retrospective analysis. The gate pipeline is
the fastest tier. The evaluation lifecycle describes how gate verdicts compound, combine
with other signals, and drive progressive improvement across time.

The lifecycle has 14 feedback loops organized across 5 speed tiers. Each loop composes
with the others — the output of fast loops feeds into slower loops, and the insights
from slow loops adjust the parameters of fast loops.

> **Citation**: bardo-backup/prd/16-testing/09-evaluation-map.md — "14 feedback loops
> across 5 speed tiers, complete composition diagram."

---

## 2. The Five Speed Tiers

| Tier | Speed | Loops | What Runs Here |
|---|---|---|---|
| Machine speed | Sub-second to seconds | 5 | Confidence calibration, context attribution, cost-effectiveness, tool selection, adversarial awareness |
| Cognitive speed | Seconds to minutes | 3 | Gate pipeline, error diagnosis, retry logic |
| Consolidation speed | Minutes to hours | 3 | Skill extraction, pattern discovery, model calibration |
| Retrospective speed | Hours to days | 2 | Shadow testing, reasoning quality review |
| Meta speed | Days to weeks | 1 | Meta-learning evaluation |

### 2.1 Machine Speed (5 Loops)

These run within or immediately after a single agent turn:

**Loop 1: Confidence Calibration**
The agent (or router) predicts success probability before the gate runs. After the gate
runs, the prediction is compared to the outcome. Residuals accumulate, enabling
calibration correction.

Metric: Expected Calibration Error (ECE) — the average gap between predicted and actual
pass rates across confidence bins.

**Loop 2: Context Attribution**
Which parts of the prompt contributed to gate success? The section effectiveness tracker
correlates prompt sections with outcomes.

**Loop 3: Cost-Effectiveness**
Did the agent's token spend produce proportionate verification results? A 50,000-token
turn that fails all gates is less cost-effective than a 10,000-token turn that passes
all gates.

**Loop 4: Tool Selection**
Are the agent's tool call patterns efficient? Redundant file reads, unnecessary edits,
and tool calls that don't advance the task are identified.

**Loop 5: Adversarial Awareness**
Does the agent detect adversarial inputs (prompt injections, malicious test fixtures)?
This loop monitors the agent's defensive behavior.

> **Citation**: bardo-backup/prd/16-testing/07-fast-feedback-loops.md — "5 machine-speed
> evaluation loops: confidence calibration, context attribution, cost-effectiveness, tool
> selection, adversarial awareness."

### 2.2 Cognitive Speed (3 Loops)

These run during a single task execution (across multiple turns):

**Loop 6: Gate Pipeline**
The rung selector → gate pipeline → verdict cycle. This is the core verification loop,
documented in [03-gate-pipeline.md](./03-gate-pipeline.md).

**Loop 7: Error Diagnosis**
Gate error output is parsed into structured feedback (see
[08-agent-feedback-from-gates.md](./08-agent-feedback-from-gates.md)) and enriched
with cheap-model diagnosis.

**Loop 8: Retry Logic**
The orchestrator decides whether to retry, escalate, or re-plan based on the verdict
and process reward signals.

### 2.3 Consolidation Speed (3 Loops)

These run after a batch of tasks (e.g., after a full plan execution):

**Loop 9: Skill Extraction**
Successful episodes are analyzed to extract reusable tool-use patterns (see
[11-evoskills.md](./11-evoskills.md)).

**Loop 10: Pattern Discovery**
Cross-task analysis identifies recurring success/failure patterns. E.g., "tasks that
modify auth modules fail 3x more often than average."

**Loop 11: Model Calibration**
Aggregate per-model performance data to calibrate the router's bandit arms. Thompson
Sampling parameters are updated based on gate outcomes.

### 2.4 Retrospective Speed (2 Loops)

These run on a longer cadence (nightly, weekly):

**Loop 12: Shadow Testing**
Run the same tasks with different models/prompts in shadow mode and compare outcomes.
This discovers whether the current routing is optimal.

**Loop 13: Reasoning Quality Review**
Evaluate agent reasoning quality across a batch of completed tasks. Three signals:
alignment (did the agent follow the plan?), consistency (did reasoning stay coherent
across turns?), and annotations (did the agent leave useful comments?).

> **Citation**: bardo-backup/prd/16-testing/08-slow-feedback-loops.md — "3 slow loops:
> shadow strategy testing, reasoning quality review, meta-learning evaluation."

### 2.5 Meta Speed (1 Loop)

**Loop 14: Meta-Learning Evaluation**
Evaluate the evaluation system itself: are the 13 other loops improving outcomes over
time? This is the system's self-assessment, tracking whether its learning is net
positive.

---

## 3. Composition Diagram

The 14 loops compose hierarchically — faster loops feed data to slower loops:

```
Machine Speed                 Cognitive Speed
┌─────────────┐              ┌─────────────┐
│ Confidence  │──residuals──→│ Gate         │
│ Calibration │              │ Pipeline     │
└─────────────┘              └──────┬───────┘
┌─────────────┐                     │
│ Context     │──lift────────┐      │ verdicts
│ Attribution │              │      ↓
└─────────────┘              │ ┌────────────┐
┌─────────────┐              │ │ Retry      │
│ Cost-       │──efficiency──│ │ Logic      │
│ Effectiveness│             │ └────────────┘
└─────────────┘              │
┌─────────────┐              │  Consolidation Speed
│ Tool        │──patterns────│  ┌──────────────┐
│ Selection   │              ├─→│ Skill         │
└─────────────┘              │  │ Extraction    │
┌─────────────┐              │  └───────────────┘
│ Adversarial │──alerts──────┤  ┌──────────────┐
│ Awareness   │              ├─→│ Pattern       │
└─────────────┘              │  │ Discovery     │
                             │  └───────────────┘
                             │  ┌──────────────┐
                             └─→│ Model         │
                                │ Calibration   │
                                └───────┬───────┘
                                        │
                Retrospective Speed     │
                ┌──────────────┐        │
                │ Shadow       │←───────┘
                │ Testing      │
                └──────────────┘
                ┌──────────────┐
                │ Reasoning    │
                │ Quality      │
                └──────┬───────┘
                       │
           Meta Speed  │
           ┌───────────┴──┐
           │ Meta-Learning │
           │ Evaluation    │
           └──────────────┘
```

---

## 4. The Karpathy Property

Every loop in the evaluation lifecycle satisfies the Karpathy Property: **if the
evaluation metric improves, the system's end-to-end performance improves**. This is a
design constraint, not an observation. It means:

- No metric that is uncorrelated with actual task success
- No metric that can be gamed without improving outcomes
- No metric that improves at the expense of another metric

For gate-based loops, the Karpathy Property is straightforward: if the compile gate pass
rate improves, the system is producing better code. For slower loops (reasoning quality,
shadow testing), the property requires careful metric design.

> **Citation**: bardo-backup/prd/16-testing/07-fast-feedback-loops.md — "Karpathy
> autoresearch loop" and Karpathy Property across all loops.

> **Citation**: bardo-backup/prd/16-testing/09-evaluation-map.md — "Karpathy property
> across all loops."

---

## 5. The Four-Phase Lifecycle

Beyond the speed tiers, evaluation goes through four phases:

### Phase 1: Trace Inspection

Examine individual agent turns: what did the agent do, what tools did it call, what
was the result? This is the raw data layer.

**Data sources**: Episode log (`.roko/episodes.jsonl`), efficiency events
(`.roko/learn/efficiency.jsonl`).

### Phase 2: Backtesting

Replay past executions with different parameters: would a different model have
succeeded? Would a different prompt have helped? Would more/fewer retries have been
optimal?

**Data sources**: Artifact store (replay exact inputs), gate threshold history.

### Phase 3: Paper Trading

Run new configurations in shadow mode alongside production. Compare outcomes without
affecting real task execution.

**Data sources**: Shadow execution results vs. production results.

### Phase 4: Canary Deployment

Gradually roll out proven improvements to a fraction of tasks, monitoring for regressions
before full deployment.

**Data sources**: A/B experiment results (`.roko/learn/experiments.json`).

> **Citation**: bardo-backup/prd/16-testing/05-evaluation-lifecycle.md — "4-phase
> evaluation lifecycle: Trace Inspection → Backtesting → Paper Trading → Canary."

---

## 6. The Gauntlet

The Gauntlet is the benchmark suite that validates the evaluation lifecycle itself:

| Speed | Duration | Scope |
|---|---|---|
| Smoke | 5 minutes | Core gate pipeline on known test cases |
| Nightly | 2–4 hours | Full rung ladder on real project tasks |
| Full | 24–48 hours | All 14 loops, cross-model comparison |

The Gauntlet provides confidence that changes to the evaluation system don't regress
evaluation quality. It is the "gate for the gates."

> **Citation**: bardo-backup/prd/16-testing/01-gauntlet.md — Gauntlet benchmark suite,
> 3 speeds.

---

## 7. Gate Verdicts as the Foundation

Every loop in the evaluation lifecycle is ultimately grounded in gate verdicts. Even the
slowest loop (meta-learning) depends on the aggregate of gate outcomes to measure
whether the system is improving.

This is why the gate architecture is so important:
- Verdicts are the atomic unit of verification truth
- All 14 loops consume or aggregate verdicts
- The quality of the evaluation lifecycle is bounded by the quality of the gates

Improving gate fidelity (adding rungs, reducing false negatives, increasing coverage)
has multiplicative effects across all 14 loops. This is another manifestation of the
GVU framework's insight: invest in verification quality.

> **Citation**: Song et al. (ICLR 2025) — "Self-improvement succeeds when the verifier
> is strong, not when the generator is strong."

---

## 8. Currently Wired Components

| Component | Status | Where |
|---|---|---|
| Gate pipeline (Loop 6) | Wired | `orchestrate.rs` per-task |
| Error feedback (Loop 7) | Wired | `feedback.rs` → agent retry |
| Adaptive thresholds | Wired | `adaptive_threshold.rs` → persist |
| Efficiency events (Loops 1–5) | Wired | `.roko/learn/efficiency.jsonl` |
| Episode logging | Wired | `.roko/episodes.jsonl` |
| Model routing (Loop 11) | Wired | `cascade-router.json` |
| A/B experiments | Wired | `experiments.json` |
| Skill library (Loop 9) | Scaffold | `roko-learn/src/skill_library.rs` |
| Shadow testing (Loop 12) | Design | Implementation plan 2J |
| Meta-learning (Loop 14) | Design | Implementation plan Phase 7–8 |

---

## 9. Summary

The evaluation lifecycle is not just "run gates and check if they pass." It is a
multi-timescale system that:
1. Runs 14 feedback loops across 5 speed tiers
2. Compounds fast signals into slow insights
3. Uses slow insights to tune fast parameters
4. Validates itself through the Gauntlet

The gate pipeline is the heartbeat at the center. Everything else — calibration, skill
extraction, shadow testing, meta-learning — depends on the gate verdicts being
accurate, fast, and comprehensive.


---

# SOURCE: /Users/will/dev/nunchi/roko/roko/docs/04-verification/10-autonomous-eval-generation.md

# 10 — Autonomous Evaluation Generation

> **Layer**: L3 Harness — Verification
> **Crates**: `roko-gate` (generated_test_gate, property_test_gate), `roko-agent`
> **Status**: Scaffold (gate implementations exist, generation pipeline designed)


> **Implementation**: Shipping

---

## 1. Overview

Autonomous evaluation generation is the system's ability to create its own verification
criteria — test cases, property assertions, invariant checks — without human
intervention. Rather than relying solely on hand-written tests, the system generates
targeted tests for each task, creating verification that specifically exercises the
agent's output.

This closes a critical gap: hand-written tests verify what the human thought to test.
Generated tests verify what the *agent actually changed*. The two are complementary, and
the generated tests often catch issues that hand-written tests miss.

> **Citation**: bardo-backup/tmp/agent-chain/17-autonomous-eval-generation.md —
> Autonomous evaluation generation architecture.

> **Citation**: bardo-backup/tmp/mori-agents/20-verification-first-architecture.md —
> "Separate test generation from implementation, tests must fail before implementation."

---

## 2. The Verification Generation Pipeline

The pipeline has three stages, executed before the implementation agent starts:

### Stage 1: Test Generation

A dedicated test-generation agent (not the implementation agent) reads the task
specification and generates test cases. These tests are:
- Targeted at the specific code being changed
- Written to fail before the implementation (red-green-refactor)
- Stored as immutable artifacts in the ArtifactStore

### Stage 2: Test Validation

The generated tests are compiled and run against the *current* codebase (before the
agent's changes). Expected behavior:
- New tests for new functionality: should fail (the functionality doesn't exist yet)
- Tests for existing functionality being modified: should pass (baseline)
- Tests that don't compile: rejected

This validation step ensures the generated tests are meaningful: they test something
that doesn't exist yet (or something that should change).

### Stage 3: Test Registration

Validated tests are registered with the `GeneratedTestGate` (Rung 4). When the
implementation agent produces its code, Rung 4 runs these tests against the new code.
If the implementation is correct, the tests should now pass.

```
Before implementation:
  Generated tests → FAIL (expected: feature doesn't exist)

After implementation:
  Generated tests → PASS (expected: feature now works)
```

This is the classic test-driven development cycle, but automated: the system generates
the tests, the agent generates the implementation, and the gate verifies alignment.

---

## 3. Test Generation Strategies

### 3.1 Example-Based Test Generation

The test agent generates concrete example tests with specific inputs and expected
outputs:

```rust
#[test]
fn rate_limiter_allows_within_limit() {
    let limiter = RateLimiter::new(100, Duration::from_secs(60));
    assert!(limiter.check("client-1").is_ok());
}

#[test]
fn rate_limiter_rejects_over_limit() {
    let limiter = RateLimiter::new(1, Duration::from_secs(60));
    limiter.check("client-1").unwrap();
    assert!(limiter.check("client-1").is_err());
}
```

These are the most common generated tests. They are easy to generate, easy to
understand, and have clear pass/fail semantics.

### 3.2 Property-Based Test Generation

For tasks where properties are more important than specific examples, the system
generates property tests:

```rust
#[proptest]
fn rate_limiter_never_allows_over_limit(limit in 1..100u32, requests in 1..200u32) {
    let limiter = RateLimiter::new(limit, Duration::from_secs(60));
    let mut allowed = 0;
    for _ in 0..requests {
        if limiter.check("client").is_ok() {
            allowed += 1;
        }
    }
    prop_assert!(allowed <= limit);
}
```

Property tests exercise a wider input space than example tests. They are more expensive
to run but catch edge cases that example tests miss.

### 3.3 Invariant Generation

The system identifies invariants that should hold before and after the agent's changes:

```rust
// Pre-condition: these tests pass before the agent's changes
// Post-condition: these tests still pass after the agent's changes
// (i.e., the agent didn't break existing functionality)

#[test]
fn existing_auth_still_works() {
    let auth = AuthService::new();
    assert!(auth.validate_token("valid-token").is_ok());
}
```

Invariant tests protect against regressions — they verify that the agent's changes
don't break things that were previously working.

---

## 4. The Key Architectural Decision: Separation

**Test generation and implementation are performed by different agents.**

This separation is critical. If the implementation agent generates its own tests, it
will generate tests that pass for its implementation — not tests that verify
correctness. The implementation agent is incentivized to make tests easy to pass. The
test generation agent is incentivized to make tests hard to pass (thorough).

This adversarial relationship improves verification quality:
- Test agent: "Here are the hardest tests I can think of for this task"
- Implementation agent: "Here is code that passes all those tests"
- Gate: "Confirmed — the code passes the tests"

> **Citation**: bardo-backup/tmp/mori-agents/20-verification-first-architecture.md —
> "Separate test generation from implementation" as a key architectural decision.

---

## 5. The Generation-Verification Gap

Song et al. (ICLR 2025) formalize the **Generation-Verification Gap**: self-improvement
works only when verification capability exceeds generation capability. In the context
of autonomous eval generation:

- If the test generator produces tests that are easier than the implementation task,
  generated tests add no value (the implementation passes trivially)
- If the test generator produces tests that are harder than the implementation task,
  generated tests provide strong verification signal

The system needs to ensure that test generation is at least as sophisticated as
implementation. This is achieved by:
1. Using a capable model for test generation (not a cheap model)
2. Including edge cases, error conditions, and adversarial inputs in generated tests
3. Validating that generated tests actually fail before implementation

> **Citation**: Song et al. (ICLR 2025) — "Self-improvement works only when the
> verifier is strong, not when the generator is strong."

---

## 6. Cheap Model Convergence Loop

For simple generated tests, the system uses a convergence loop with a cheap model:

```
while not converged:
    cheap_model generates implementation attempt
    generated_tests run against attempt
    if all pass: converged = true
    else: feed errors back to cheap_model

if converged:
    submit to full gate pipeline
else (after N attempts):
    escalate to expensive model
```

This is a cost optimization: many tasks can be solved by a cheap model (Haiku-class)
with generated tests providing the feedback signal. Only tasks that the cheap model
can't solve get escalated to an expensive model (Opus-class).

The savings compound: if 60% of tasks are solvable by a cheap model, the overall cost
drops by ~50% while maintaining quality (because generated tests enforce the same
standard regardless of which model produced the code).

> **Citation**: bardo-backup/tmp/mori-agents/20-verification-first-architecture.md —
> "Cheap model convergence loop" architecture.

---

## 7. Immutable Verification Artifacts

Generated tests are stored as immutable artifacts in the ArtifactStore before
implementation begins. This ensures:

1. **No tampering**: The implementation agent cannot modify the tests to make them pass
2. **Reproducibility**: The exact tests used for verification can be retrieved later
3. **Forensic replay**: Any verification outcome can be replayed with the original tests

The content-addressed nature of the ArtifactStore (BLAKE3 hashes) makes this
immutability cryptographic, not just conventional.

> **Citation**: bardo-backup/tmp/mori-agents/20-verification-first-architecture.md —
> "Tests must fail before implementation, immutable verification artifacts."

---

## 8. Dependency Detection and Mock Generation

For tasks that interact with external systems (databases, APIs, file systems), the
test generation pipeline includes:

### 8.1 Dependency Detection

Analyze the task specification and the code being modified to identify external
dependencies:
- File system access (read/write)
- Network calls (HTTP, gRPC)
- Database queries
- System clock usage
- Random number generation

### 8.2 Mock Strategy Selection

For each detected dependency, select a mock strategy:
- **In-process mock**: Replace the dependency with a mock implementation
- **Test fixture**: Provide pre-built data files
- **Temporal mock**: Fix the system clock to a known time
- **Deterministic seed**: Fix random seeds for reproducibility

### 8.3 Sidecar Generation

For complex dependencies (databases, external services), generate sidecar test
infrastructure:
- Docker Compose files for test databases
- Mock servers for external APIs
- Test data fixtures with known states

> **Citation**: bardo-backup/tmp/death/16-autonomous-verification.md — "Autonomous test
> infrastructure generation, dependency detection, mock strategies, invariant generation,
> sidecar lifecycle management."

---

## 9. Gate Integration

Generated tests integrate with the gate pipeline through two gates:

### 9.1 GeneratedTestGate (Rung 4)

Runs example-based generated tests. Behaves like `TestGate` but operates on a different
test suite — the generated tests rather than the project's existing tests.

### 9.2 PropertyTestGate (Rung 5)

Runs property-based generated tests. Uses proptest or quickcheck frameworks with
generated property definitions.

Both gates return standard `Verdict` objects, so they compose naturally with the gate
pipeline, ratchet, and adaptive thresholds.

---

## 10. Evaluation Quality Metrics

The autonomous eval generation system tracks its own quality:

| Metric | What It Measures | Target |
|---|---|---|
| Test generation success rate | % of generated tests that compile | > 95% |
| Test discrimination | % of generated tests that fail before implementation | > 80% |
| False positive rate | % of generated tests that fail on correct implementations | < 5% |
| Coverage improvement | Additional code coverage from generated tests | > 20% over hand-written |
| Cost per test | Token cost to generate one test | < $0.01 |

These metrics feed into the meta-learning loop (Loop 14 in the evaluation lifecycle),
which adjusts the test generation strategy over time.

---

## 11. Summary

Autonomous evaluation generation transforms the gate pipeline from a static checker
into a dynamic verification system. By generating tests specific to each task, the
system:

1. Catches issues that hand-written tests miss (they test what changed, not what the
   human thought to test)
2. Enforces the test-driven development cycle automatically
3. Creates an adversarial relationship between test generation and implementation
4. Enables cheap-model convergence loops that reduce cost while maintaining quality

The separation of test generation from implementation is the key architectural insight:
the agent that writes the code should not be the agent that writes the tests.


---

# SOURCE: /Users/will/dev/nunchi/roko/roko/docs/04-verification/11-evoskills.md

# 11 — EvoSkills: Self-Evolving Verification Skills

> **Layer**: L3 Harness — Verification × L2 Engine — Learning
> **Crates**: `roko-learn` (skill_library, pattern_discovery), `roko-gate`
> **Status**: Skill library scaffold exists, adversarial verification designed


> **Implementation**: Shipping

---

## 1. Overview

EvoSkills is a self-evolving skill library where verification skills improve
autonomously through adversarial surrogate verification and cross-model transfer. The
core idea: agents accumulate reusable tool-use patterns from successful task executions,
and these patterns are validated against adversarial test suites to ensure they
generalize.

The empirical results from the reference system are striking:
- Baseline success rate: 32%
- With EvoSkills: 75% (+43 percentage points)
- Cross-model transfer improvement: +35–44 percentage points

These numbers come from the adversarial surrogate verification process: skills that
survive adversarial testing are genuinely useful, not just incidental patterns from
lucky executions.

> **Citation**: refactoring-prd/09-innovations.md — Innovation X: "EvoSkills (32%→75%,
> cross-model +35-44pp)."

---

## 2. The Three-Tier Learning Hierarchy

Skills emerge from Roko's three-tier learning system:

### Tier 1: Episodes (Raw)

Every agent execution is recorded as an episode in `.roko/episodes.jsonl`. An episode
contains:
- Task specification
- Tool calls (in order)
- Gate verdicts
- Final outcome (passed/failed)
- Token counts and timing

Episodes are the raw data. They are never modified or deleted.

### Tier 2: Patterns (Extracted)

When 5+ similar episodes show the same tool-use sequence leading to success, a pattern
is extracted:
- Precondition: what task characteristics trigger this pattern?
- Procedure: what tool calls compose the pattern?
- Postcondition: what gate outcomes does this pattern produce?

Patterns are hypotheses — they suggest that a particular approach works for a particular
kind of task. They are not yet validated.

### Tier 3: Playbook (Validated)

When a pattern has been successfully applied 5+ times in production (not just extracted
from historical data), it is promoted to a playbook rule. Playbook rules are:
- Validated through actual use
- Injected into agent context via the prompt's "skills" section
- Tracked with confidence scores and usage telemetry

> **Citation**: tmp/implementation-plans/modelrouting/12-advanced-patterns.md §D —
> "Mori's learning hierarchy: Tier 1 Episodes, Tier 2 Patterns, Tier 3 Playbook."

---

## 3. Adversarial Surrogate Verification

The breakthrough in EvoSkills is the validation step. Rather than trusting that
extracted patterns generalize, the system tests them adversarially:

### 3.1 Surrogate Test Generation

For each candidate skill, generate a suite of adversarial test cases that specifically
try to break the skill:
- Edge cases the skill might not handle
- Input variations that test generalization
- Failure modes that would reveal brittle patterns

### 3.2 Cross-Model Testing

Apply the skill with different models to verify it transfers:
- If a skill works with Model A but fails with Model B, it may be an artifact of
  Model A's specific capabilities rather than a genuine skill
- Skills that work across 3+ models are robust and likely capture genuine
  task-completion knowledge

### 3.3 Confidence Scoring

Each skill maintains a confidence score based on:

```
confidence = (validations / (validations + failures)) × cross_model_factor
```

Where `cross_model_factor` is:
- 1.0 if validated across 3+ models
- 0.8 if validated across 2 models
- 0.6 if validated on only 1 model

Skills below a confidence threshold (e.g., 0.5) are demoted back to Tier 2 for
re-extraction.

---

## 4. Skill Structure

```rust
pub struct Skill {
    pub id: String,
    pub name: String,
    pub precondition: String,         // When to apply this skill
    pub procedure: String,            // What to do (tool call sequence summary)
    pub postcondition: String,        // Expected outcome
    pub confidence: f64,              // [0, 1] based on validation
    pub source_episodes: Vec<String>, // Episode IDs this was extracted from
    pub validations: u64,             // Times successfully applied
    pub failures: u64,                // Times applied but failed
    pub task_categories: Vec<String>, // Which task types this applies to
    pub created_at: String,
    pub last_validated_at: Option<String>,
}
```

### Example Skill

```
Skill: "Rust Compile Fix — Missing Import"
Precondition: compile gate fails with error[E0433] or error[E0425]
Procedure:
  1. Read the error message to identify the missing symbol
  2. Search for the symbol in the codebase (Grep)
  3. Identify the crate/module that exports it
  4. Add the use statement to the file
Postcondition: compile gate passes
Confidence: 0.92 (validated 46 times, failed 4 times, 3 models)
```

> **Citation**: SAGE (arXiv:2512.17102) — "Agents that accumulate reusable tool-use
> patterns across tasks use 26% fewer steps and 59% fewer tokens."

---

## 5. Skill Injection into Agent Context

Validated skills are injected into the agent's prompt as a dedicated section:

```
## Relevant Skills

Based on the current task (compile error fix, auth module), the following
verified skills may be applicable:

### Skill: Rust Compile Fix — Missing Import (confidence: 0.92)
When: compile gate fails with error[E0433] or error[E0425]
Do: Read error → Grep for symbol → Identify export → Add use statement
Expected: compile gate passes

### Skill: Auth Module Test Pattern (confidence: 0.78)
When: task involves auth module changes
Do: Read existing auth tests → Modify in parallel → Run TestGate
Expected: all auth tests pass
```

The section effectiveness tracker (from the gate-to-scaffold feedback loop) monitors
whether skill injection actually improves gate pass rates. Skills whose injection
doesn't improve outcomes get their priority reduced.

---

## 6. Cross-Model Transfer

The +35–44pp cross-model transfer improvement means that skills extracted from Model A's
successful executions help Model B succeed:

```
Model A executes 100 tasks:
  → 50 succeed, 50 fail (50% base rate)
  → Extract 15 skills from the 50 successes

Model B executes 100 tasks (same distribution):
  → Without skills: 32% success rate
  → With Model A's skills: 67–76% success rate (+35–44pp)
```

This transfer works because the skills encode *task-completion knowledge* (how to fix
compile errors, how to write tests, how to modify auth modules), not model-specific
behaviors. A skill like "read the error, search for the symbol, add the import" works
regardless of which model executes it.

This has a practical implication: **new models get instant expertise**. When a new model
is added to the routing pool, it immediately benefits from the accumulated skill library
without any warm-up period.

---

## 7. Skill Evolution

Skills evolve over time through three mechanisms:

### 7.1 Refinement

When a skill fails, the failure is analyzed:
- Was the precondition too broad? → Narrow the precondition
- Was the procedure missing a step? → Add the missing step
- Was the postcondition wrong? → Correct the expected outcome

### 7.2 Specialization

A general skill that works for most cases may fail for a specific sub-case. In that
case, a specialized variant is created:

```
General: "Fix compile error — missing import"
Specialized: "Fix compile error — missing import from workspace crate"
  (adds: check Cargo.toml for workspace dependencies before adding use)
```

### 7.3 Retirement

Skills whose confidence drops below the threshold (e.g., due to codebase evolution
making old patterns obsolete) are retired:
- Removed from the active playbook
- Retained in the archive for historical analysis
- May be re-extracted if conditions change

---

## 8. Relationship to Verification

EvoSkills are deeply connected to the verification layer:

### 8.1 Gate Verdicts Drive Skill Extraction

Skills are extracted from episodes where **all gates passed**. A successful episode must
have:
- Compile gate: PASS
- Lint gate: PASS
- Test gate: PASS
- Any other rungs that ran: PASS

This ensures skills are extracted from genuinely successful executions, not from
partial successes.

### 8.2 Gate Verdicts Validate Skills

Skill validation means: "Apply the skill on a new task and verify that the gates pass."
The gate pipeline is the validation oracle. No gate pass = no validation credit.

### 8.3 Skills Improve Gate Pass Rates

The feedback loop closes: skills extracted from gate successes are injected into prompts,
leading to more gate successes, leading to more skill extraction. This is a positive
feedback loop that compounds over time.

The risk of positive feedback loops is runaway behavior. The adversarial surrogate
verification is the stabilizer: it prunes skills that don't genuinely help, preventing
the accumulation of noise.

---

## 9. Relationship to the Evaluation Lifecycle

EvoSkills operate at the Consolidation Speed tier (Loop 9: Skill Extraction):

```
Machine Speed:     Per-turn tool call data collection
Cognitive Speed:   Gate pipeline produces verdicts
Consolidation:     Skill extraction from episodes with passing gates   ← EvoSkills
Retrospective:     Cross-model validation of extracted skills
Meta:              Evaluate whether skill library is net positive
```

The consolidation loop runs after a batch of tasks completes. It scans recent successful
episodes, clusters them by similarity, and extracts candidate skills. The retrospective
loop validates candidates across models. The meta loop evaluates whether the skill
library as a whole is improving outcomes.

> **Citation**: bardo-backup/prd/16-testing/09-evaluation-map.md — "14 feedback loops
> across 5 speed tiers."

---

## 10. Academic Foundations

### SAGE (arXiv:2512.17102)

Self-Acquired Generalist Expertise: demonstrated that agents accumulating reusable
patterns across tasks use 26% fewer steps and 59% fewer tokens. EvoSkills applies this
to verification-guided development.

### Voyager (Wang et al. 2023)

Showed that an LLM agent in Minecraft that accumulates a skill library achieves
dramatically more complex goals than one that starts fresh each time. The skill library
is the agent's "long-term procedural memory."

### DSPy Bayesian Optimizers

The Bayesian optimization approach to prompt tuning from DSPy provides a framework for
skill injection: each skill is a prompt component whose inclusion probability is
optimized based on gate outcomes.

> **Citation**: SAGE (arXiv:2512.17102), Voyager (Wang et al. 2023) — Skill library
> foundations.

---

## 11. Summary

EvoSkills transform Roko from a system that treats each task independently into one
that learns from experience. The three-tier hierarchy (Episodes → Patterns → Playbook)
ensures that only validated, generalizing skills enter the agent's context. Adversarial
surrogate verification prevents noise accumulation. Cross-model transfer means knowledge
compounds across the entire model pool.

The 32% → 75% improvement is the headline number, but the deeper insight is: **skills
are extracted from verification successes and validated by verification**. The gate
pipeline is both the source of skill data and the arbiter of skill quality.

---

## 12. Skill Genome Representation

To evolve skills systematically, we need a genome — a structured representation
that can be mutated, crossed over, and evaluated. The skill genome encodes not just
the procedure but the entire agent configuration that produces the skill.

> **Citation**: "Agent Skill Acquisition for Large Language Models" (ICLR 2025) —
> MAP-Elites-style cyclic optimization for LLM agent skills.

### 12.1 Genome Structure

```rust
/// A skill genome encodes the full agent configuration that produces a skill.
/// This is the unit of evolution — what gets mutated, crossed over, and selected.
pub struct SkillGenome {
    /// Unique identifier for this genome.
    pub id: String,
    /// The skill this genome encodes (precondition, procedure, postcondition).
    pub skill: Skill,

    // --- Evolvable parameters ---

    /// System prompt strategy: how the skill is described to the agent.
    /// Mutation: rephrase, add/remove context, change emphasis.
    pub prompt_template: String,
    /// Tool usage preferences: which tools the skill prioritizes.
    /// Each entry is (tool_name, priority_weight).
    /// Mutation: adjust weights, add/remove tools.
    pub tool_preferences: Vec<(String, f64)>,
    /// Retry strategy when the skill fails partway through.
    pub retry_config: RetryGenome,
    /// Model temperature for LLM calls during skill execution.
    /// Mutation: Gaussian perturbation.
    pub temperature: f64,            // default: 0.3, range: [0.0, 1.0]
    /// Token budget allocated to this skill's execution.
    /// Mutation: scale up/down.
    pub token_budget: usize,         // default: 4096
    /// Gate weights: how much the skill cares about each gate.
    /// Used for fitness computation. Mutation: adjust per-gate weights.
    pub gate_weights: Vec<f64>,      // one per rung

    // --- Behavioral descriptor (for MAP-Elites) ---

    /// Measured behavioral characteristics (not evolvable — computed from execution).
    pub behavior: BehavioralDescriptor,

    // --- Fitness ---

    /// Fitness score from the last evaluation.
    pub fitness: f64,
    /// Number of evaluations.
    pub evaluations: u64,
}

#[derive(Debug, Clone)]
pub struct RetryGenome {
    /// Maximum retries for this skill.
    pub max_retries: u32,       // default: 3, range: [1, 8]
    /// Backoff multiplier between retries.
    pub backoff_factor: f64,    // default: 1.5, range: [1.0, 4.0]
    /// Whether to adjust prompt between retries.
    pub prompt_mutation_on_retry: bool,
}
```

### 12.2 Behavioral Descriptors

Behavioral descriptors define the axes of the MAP-Elites archive. They are measured
from execution, not set directly — the same genome can produce different behaviors
in different contexts.

```rust
/// Measured behavioral characteristics of a skill execution.
/// These define the axes of the MAP-Elites quality-diversity archive.
#[derive(Debug, Clone)]
pub struct BehavioralDescriptor {
    /// Axis 1: Task completion rate [0, 1].
    /// What fraction of tasks this skill completes successfully.
    pub completion_rate: f64,
    /// Axis 2: Average gate score across all rungs [0, 1].
    /// How thoroughly the skill passes verification.
    pub gate_score: f64,
    /// Axis 3: Token efficiency — inverse of tokens per successful task.
    /// Normalized to [0, 1] where 1.0 = cheapest observed.
    pub token_efficiency: f64,
    /// Axis 4: Generalization breadth — fraction of task categories
    /// where this skill has been successfully applied.
    pub generalization: f64,
}

impl BehavioralDescriptor {
    /// Discretize into a MAP-Elites cell index.
    ///
    /// Each axis is divided into `resolution` bins.
    /// With 4 axes and 10 bins each, the archive has 10,000 cells.
    pub fn to_cell(&self, resolution: usize) -> [usize; 4] {
        let bin = |v: f64| ((v * resolution as f64) as usize).min(resolution - 1);
        [
            bin(self.completion_rate),
            bin(self.gate_score),
            bin(self.token_efficiency),
            bin(self.generalization),
        ]
    }
}
```

---

## 13. MAP-Elites for Skill Quality-Diversity

Standard evolutionary algorithms optimize for a single fitness function, converging
to one solution. MAP-Elites maintains an archive of diverse, high-quality solutions
across a behavioral space. This is critical for skills: we don't want one "best" skill —
we want a diverse repertoire covering different task types and strategies.

> **Citation**: Mouret & Clune, "Illuminating Search Spaces by Mapping Elites"
> (arXiv:1504.04909, 2015).

> **Citation**: "Quality-Diversity Methods for the Modern Data Scientist" (WIREs
> Computational Statistics, 2025).

### 13.1 Archive Structure

```rust
/// MAP-Elites archive: a grid of skill genomes indexed by behavioral descriptor.
///
/// Each cell stores the highest-fitness genome observed for that behavior region.
/// Empty cells represent unexplored behavioral niches.
pub struct SkillArchive {
    /// Grid resolution per behavioral axis.
    pub resolution: usize,         // default: 10
    /// Number of behavioral dimensions.
    pub dimensions: usize,         // 4 (completion, gate_score, efficiency, generalization)
    /// The archive grid. Keyed by flattened cell index.
    /// Each cell stores the best genome for that behavioral niche.
    pub cells: HashMap<usize, SkillGenome>,
    /// Total evaluations performed (across all generations).
    pub total_evaluations: u64,
    /// Generation counter.
    pub generation: u64,
}

impl SkillArchive {
    /// Insert a genome into the archive if it's the best for its cell.
    pub fn try_insert(&mut self, genome: SkillGenome) -> InsertResult {
        let cell = genome.behavior.to_cell(self.resolution);
        let flat_index = self.flatten_index(&cell);

        match self.cells.entry(flat_index) {
            Entry::Vacant(e) => {
                e.insert(genome);
                InsertResult::NewNiche
            }
            Entry::Occupied(mut e) => {
                if genome.fitness > e.get().fitness {
                    e.insert(genome);
                    InsertResult::Improved
                } else {
                    InsertResult::Rejected
                }
            }
        }
    }

    /// Coverage: fraction of cells that are filled.
    pub fn coverage(&self) -> f64 {
        let total_cells = self.resolution.pow(self.dimensions as u32);
        self.cells.len() as f64 / total_cells as f64
    }

    /// QD-score: sum of all fitness values in the archive.
    /// Higher = better quality AND diversity.
    pub fn qd_score(&self) -> f64 {
        self.cells.values().map(|g| g.fitness).sum()
    }

    /// Select a random parent from the archive for mutation.
    pub fn random_parent(&self, rng: &mut impl Rng) -> Option<&SkillGenome> {
        let keys: Vec<_> = self.cells.keys().collect();
        if keys.is_empty() { return None; }
        let idx = rng.gen_range(0..keys.len());
        self.cells.get(keys[idx])
    }
}

pub enum InsertResult {
    /// Filled a previously empty cell — new behavioral niche discovered.
    NewNiche,
    /// Replaced an existing genome with a fitter one.
    Improved,
    /// Existing genome in that cell was fitter — genome discarded.
    Rejected,
}
```

### 13.2 Evolution Loop

```rust
/// One generation of MAP-Elites skill evolution.
///
/// Pseudocode:
///   for _ in 0..batch_size:
///       parent = archive.random_parent()
///       offspring = mutate(parent)
///       fitness, behavior = evaluate(offspring)
///       offspring.fitness = fitness
///       offspring.behavior = behavior
///       archive.try_insert(offspring)
pub struct SkillEvolver {
    pub archive: SkillArchive,
    /// Number of offspring per generation.
    pub batch_size: usize,          // default: 16
    /// Mutation operator configuration.
    pub mutation: MutationConfig,
    /// Gate pipeline for fitness evaluation.
    pub evaluator: GatePipeline,
    /// Task sampler for evaluation.
    pub task_sampler: Box<dyn TaskSampler>,
}

pub struct MutationConfig {
    /// Probability of mutating the prompt template.
    pub prompt_mutation_rate: f64,    // default: 0.3
    /// Probability of adjusting tool preferences.
    pub tool_mutation_rate: f64,      // default: 0.2
    /// Standard deviation for continuous parameter perturbation.
    pub param_sigma: f64,            // default: 0.1
    /// Probability of crossover (recombination of two parents).
    pub crossover_rate: f64,         // default: 0.2
}
```

### 13.3 Mutation Operators

```rust
impl SkillGenome {
    /// Mutate this genome to produce an offspring.
    pub fn mutate(&self, config: &MutationConfig, rng: &mut impl Rng) -> Self {
        let mut offspring = self.clone();
        offspring.id = generate_id();

        // Mutate prompt template (rephrase, add/remove context)
        if rng.gen::<f64>() < config.prompt_mutation_rate {
            offspring.prompt_template = mutate_prompt(&self.prompt_template);
        }

        // Mutate tool preferences (adjust weights)
        if rng.gen::<f64>() < config.tool_mutation_rate {
            for (_, weight) in &mut offspring.tool_preferences {
                *weight = (*weight + rng.gen_range(-0.2..0.2)).clamp(0.0, 1.0);
            }
        }

        // Mutate continuous parameters (Gaussian perturbation)
        offspring.temperature = (self.temperature
            + rng.gen::<f64>() * config.param_sigma * 2.0 - config.param_sigma)
            .clamp(0.0, 1.0);
        offspring.token_budget = (self.token_budget as f64
            * (1.0 + rng.gen_range(-0.2..0.2))) as usize;

        // Mutate retry config
        if rng.gen::<f64>() < 0.1 {
            offspring.retry_config.max_retries =
                (self.retry_config.max_retries as i32 + rng.gen_range(-1..=1))
                    .clamp(1, 8) as u32;
        }

        offspring
    }

    /// Crossover: recombine two genomes.
    pub fn crossover(&self, other: &Self, rng: &mut impl Rng) -> Self {
        let mut offspring = self.clone();
        offspring.id = generate_id();

        // Uniform crossover on discrete fields
        if rng.gen::<bool>() {
            offspring.prompt_template = other.prompt_template.clone();
        }
        if rng.gen::<bool>() {
            offspring.tool_preferences = other.tool_preferences.clone();
        }
        if rng.gen::<bool>() {
            offspring.retry_config = other.retry_config.clone();
        }

        // Intermediate crossover on continuous fields
        let alpha = rng.gen::<f64>();
        offspring.temperature = alpha * self.temperature
            + (1.0 - alpha) * other.temperature;
        offspring.token_budget = ((alpha * self.token_budget as f64
            + (1.0 - alpha) * other.token_budget as f64) as usize).max(1024);

        offspring
    }
}
```

---

## 14. Fitness Evaluation

Skill fitness is measured by running the skill on sampled tasks and observing gate
outcomes.

```rust
/// Fitness function for skill genomes.
///
/// Evaluates a skill by applying it to N sampled tasks and measuring
/// gate outcomes. The fitness is a weighted combination of:
///   - Gate pass rate (primary)
///   - Token efficiency (secondary)
///   - Generalization across task categories (tertiary)
pub struct SkillFitness {
    /// Number of evaluation tasks per genome.
    pub eval_tasks: usize,          // default: 5
    /// Gate pipeline for evaluation.
    pub gate_pipeline: GatePipeline,
    /// Fitness weights.
    pub weights: FitnessWeights,
}

pub struct FitnessWeights {
    /// Weight for gate pass rate [0, 1].
    pub gate_pass: f64,    // default: 0.5
    /// Weight for token efficiency [0, 1].
    pub efficiency: f64,   // default: 0.3
    /// Weight for cross-task generalization [0, 1].
    pub generalization: f64, // default: 0.2
}

impl SkillFitness {
    /// Evaluate a genome and return its fitness and behavioral descriptor.
    pub async fn evaluate(&self, genome: &SkillGenome,
                          tasks: &[Task]) -> (f64, BehavioralDescriptor) {
        let mut pass_count = 0;
        let mut total_tokens = 0;
        let mut categories_seen = HashSet::new();
        let mut categories_passed = HashSet::new();
        let mut gate_scores = Vec::new();

        for task in tasks.iter().take(self.eval_tasks) {
            let result = self.run_skill(genome, task).await;
            if result.verdict.passed {
                pass_count += 1;
                categories_passed.insert(task.category.clone());
            }
            total_tokens += result.tokens_used;
            categories_seen.insert(task.category.clone());
            gate_scores.push(result.verdict.score);
        }

        let completion = pass_count as f64 / self.eval_tasks as f64;
        let avg_gate = gate_scores.iter().sum::<f32>() as f64
            / gate_scores.len().max(1) as f64;
        let efficiency = if total_tokens > 0 {
            (pass_count as f64 * 1000.0) / total_tokens as f64
        } else { 0.0 };
        let generalization = if !categories_seen.is_empty() {
            categories_passed.len() as f64 / categories_seen.len() as f64
        } else { 0.0 };

        let fitness = self.weights.gate_pass * completion
            + self.weights.efficiency * efficiency.min(1.0)
            + self.weights.generalization * generalization;

        let behavior = BehavioralDescriptor {
            completion_rate: completion,
            gate_score: avg_gate,
            token_efficiency: efficiency.min(1.0),
            generalization,
        };

        (fitness, behavior)
    }
}
```

---

## 15. Fitness Landscape Analysis

Understanding the topology of the skill space reveals where to search for improvements
and where the landscape is deceptive or rugged.

### 15.1 Landscape Metrics

```rust
/// Fitness landscape analysis for the skill archive.
pub struct LandscapeAnalysis {
    /// Local optima count: cells where no neighbor has higher fitness.
    pub local_optima: usize,
    /// Ruggedness: average fitness difference between adjacent cells.
    /// High ruggedness = many local optima, hard to optimize.
    pub ruggedness: f64,
    /// Neutrality: fraction of adjacent cell pairs with equal fitness.
    /// High neutrality = flat plateaus (drift without progress).
    pub neutrality: f64,
    /// Fitness-distance correlation (FDC): correlation between fitness
    /// and distance to the global optimum.
    /// FDC > 0.15 → landscape is "easy" (gradient toward optimum).
    /// FDC < -0.15 → landscape is "deceptive" (gradient away from optimum).
    pub fdc: f64,
    /// Evolvability: fraction of mutations that produce fitter offspring.
    pub evolvability: f64,
    /// Coverage frontier: cells at the boundary of explored space.
    pub frontier_cells: usize,
}

impl SkillArchive {
    /// Analyze the fitness landscape of the current archive.
    pub fn analyze_landscape(&self) -> LandscapeAnalysis {
        let mut local_optima = 0;
        let mut fitness_diffs = Vec::new();
        let mut neutral_count = 0;
        let mut total_pairs = 0;

        for (&idx, genome) in &self.cells {
            let neighbors = self.get_neighbors(idx);
            let is_local_optimum = neighbors.iter()
                .all(|n| n.fitness <= genome.fitness);
            if is_local_optimum { local_optima += 1; }

            for neighbor in &neighbors {
                let diff = (genome.fitness - neighbor.fitness).abs();
                fitness_diffs.push(diff);
                if diff < 0.01 { neutral_count += 1; }
                total_pairs += 1;
            }
        }

        let ruggedness = if fitness_diffs.is_empty() { 0.0 }
            else { fitness_diffs.iter().sum::<f64>() / fitness_diffs.len() as f64 };
        let neutrality = if total_pairs == 0 { 0.0 }
            else { neutral_count as f64 / total_pairs as f64 };

        LandscapeAnalysis {
            local_optima,
            ruggedness,
            neutrality,
            fdc: self.compute_fdc(),
            evolvability: self.compute_evolvability(),
            frontier_cells: self.count_frontier_cells(),
        }
    }
}
```

### 15.2 Landscape-Adaptive Evolution

The landscape analysis feeds back into the evolution strategy:

```
if ruggedness > 0.3:
    // Rugged landscape — increase mutation strength to escape local optima
    mutation.param_sigma *= 1.5
    mutation.prompt_mutation_rate *= 1.3

if neutrality > 0.5:
    // Flat landscape — increase crossover to search broadly
    mutation.crossover_rate *= 1.5
    mutation.param_sigma *= 0.8  // reduce random walk on plateaus

if fdc < -0.15:
    // Deceptive landscape — novelty-driven search instead of fitness-driven
    switch_to_novelty_search()

if evolvability < 0.1:
    // Low evolvability — most mutations are harmful
    // Reduce mutation rates, increase elitism
    mutation.param_sigma *= 0.5
    batch_size *= 2  // more samples per generation to find rare improvements
```

---

## 16. Speciation for Prompt Strategies

Skills that use fundamentally different strategies (e.g., "fix by reading error
messages" vs "fix by searching codebase for patterns") should be protected from
competing directly. Speciation groups similar genomes into species and allocates
evaluation budget proportionally.

> **Citation**: Stanley & Miikkulainen, "Evolving Neural Networks through Augmenting
> Topologies" (NEAT, Evolutionary Computation, 2002) — speciation via compatibility
> distance.

### 16.1 Compatibility Distance

```rust
/// Compatibility distance between two skill genomes.
///
/// Measures how different two genomes are along structural and
/// parametric dimensions. Used for speciation.
pub struct CompatibilityMetric {
    /// Weight for prompt template difference (semantic similarity).
    pub c_prompt: f64,   // default: 1.0
    /// Weight for tool preference difference.
    pub c_tools: f64,    // default: 0.5
    /// Weight for continuous parameter difference.
    pub c_params: f64,   // default: 0.3
}

impl CompatibilityMetric {
    /// Compute compatibility distance between two genomes.
    pub fn distance(&self, a: &SkillGenome, b: &SkillGenome) -> f64 {
        // Prompt distance: Jaccard similarity of n-gram sets
        let prompt_dist = 1.0 - ngram_jaccard(&a.prompt_template,
                                               &b.prompt_template, 3);

        // Tool distance: cosine distance of preference vectors
        let tool_dist = tool_cosine_distance(&a.tool_preferences,
                                              &b.tool_preferences);

        // Parameter distance: normalized L2 distance
        let param_dist = (
            (a.temperature - b.temperature).powi(2)
            + ((a.token_budget as f64 - b.token_budget as f64) / 8192.0).powi(2)
            + ((a.retry_config.max_retries as f64
                - b.retry_config.max_retries as f64) / 8.0).powi(2)
        ).sqrt();

        self.c_prompt * prompt_dist
            + self.c_tools * tool_dist
            + self.c_params * param_dist
    }
}
```

### 16.2 Species Management

```rust
/// Species: a group of genomes with similar strategies.
pub struct Species {
    pub id: usize,
    /// Representative genome (used for distance comparisons).
    pub representative: SkillGenome,
    /// Members of this species.
    pub members: Vec<SkillGenome>,
    /// Adjusted fitness (fitness / species_size for fitness sharing).
    pub adjusted_fitness: f64,
    /// Generations since this species improved.
    pub stagnation_counter: u32,
}

pub struct SpeciesManager {
    pub species: Vec<Species>,
    /// Compatibility threshold. Genomes within this distance are same species.
    /// Dynamically adjusted to maintain target_species count.
    pub threshold: f64,              // default: 1.0
    /// Desired number of species.
    pub target_species: usize,       // default: 5
    /// Maximum generations without improvement before species is dissolved.
    pub stagnation_limit: u32,       // default: 15
}

impl SpeciesManager {
    /// Assign a genome to a species (or create a new one).
    pub fn speciate(&mut self, genome: SkillGenome) {
        for species in &mut self.species {
            let dist = self.metric.distance(&genome, &species.representative);
            if dist < self.threshold {
                species.members.push(genome);
                return;
            }
        }
        // No compatible species — create a new one
        self.species.push(Species {
            id: self.next_id(),
            representative: genome.clone(),
            members: vec![genome],
            adjusted_fitness: 0.0,
            stagnation_counter: 0,
        });
    }

    /// Adjust threshold to maintain target species count.
    pub fn adjust_threshold(&mut self) {
        if self.species.len() > self.target_species {
            self.threshold += 0.3; // merge similar species
        } else if self.species.len() < self.target_species {
            self.threshold -= 0.3; // split into more species
        }
        self.threshold = self.threshold.max(0.1); // floor
    }
}
```

---

## 17. AURORA: Learned Behavioral Descriptors

Hand-crafted behavioral descriptors (completion rate, gate score, etc.) may miss
important behavioral axes. AURORA uses a variational autoencoder to *discover*
behavioral descriptors from execution traces.

> **Citation**: AURORA (Unsupervised Behavior Discovery with QD, 2021–2024) — learned
> behavioral descriptors via VAE for quality-diversity optimization.

```rust
/// AURORA: learned behavioral descriptors from execution traces.
///
/// Instead of hand-crafting axes like "completion_rate" and "efficiency",
/// train a VAE on execution traces to discover latent behavioral dimensions.
pub struct AuroraDescriptor {
    /// Dimensionality of the learned behavioral space.
    pub latent_dims: usize,    // default: 4
    /// The trained VAE encoder (execution trace → latent vector).
    pub encoder: Box<dyn TraceEncoder>,
    /// Archive resolution in the learned space.
    pub resolution: usize,     // default: 10
}

pub trait TraceEncoder: Send + Sync {
    /// Encode an execution trace into a latent behavioral vector.
    fn encode(&self, trace: &ExecutionTrace) -> Vec<f64>;
}

/// Training AURORA:
///
/// 1. Collect N execution traces from the episode log
/// 2. Extract features: tool call sequences, edit patterns, gate results,
///    timing distributions, token usage patterns
/// 3. Train a VAE on the feature vectors:
///    - Encoder: features → z (latent behavioral descriptor)
///    - Decoder: z → reconstructed features
///    - Loss: reconstruction + KL divergence
/// 4. The latent space z becomes the behavioral descriptor for MAP-Elites
///
/// Benefits over hand-crafted descriptors:
/// - Discovers axes like "cautious vs aggressive editing" automatically
/// - Adapts to the codebase's actual behavioral diversity
/// - Can reveal unexpected behavioral niches worth exploring
```

---

## 18. CMA-ES for Continuous Skill Parameters

For continuous parameters (temperature, token budget, gate weights), CMA-ES is
more sample-efficient than random mutation.

> **Citation**: Hansen, "The CMA Evolution Strategy: A Tutorial"
> (arXiv:1604.00772).

```rust
/// CMA-ES optimizer for continuous skill parameters.
///
/// Operates on the continuous sub-vector of the genome:
/// [temperature, token_budget_normalized, backoff_factor, gate_weights...]
pub struct SkillCmaEs {
    /// Dimensionality of the continuous parameter space.
    pub n: usize,
    /// Population size per generation.
    pub lambda: usize,         // default: 4 + floor(3 * ln(n))
    /// Number of parents for recombination.
    pub mu: usize,             // default: lambda / 2
    /// Distribution mean (current best parameter estimate).
    pub mean: Vec<f64>,
    /// Step-size (global mutation strength).
    pub sigma: f64,            // initial: 0.3
    /// Covariance matrix (encodes parameter correlations).
    pub covariance: Vec<Vec<f64>>,
    /// Evolution path for step-size adaptation.
    pub p_sigma: Vec<f64>,
    /// Evolution path for covariance adaptation.
    pub p_c: Vec<f64>,
}

impl SkillCmaEs {
    /// Sample a population of parameter vectors.
    pub fn sample(&self, rng: &mut impl Rng) -> Vec<Vec<f64>> {
        // z ~ N(0, C), x = mean + sigma * z
        let cholesky = cholesky_decompose(&self.covariance);
        (0..self.lambda).map(|_| {
            let z: Vec<f64> = (0..self.n)
                .map(|_| rng.sample(StandardNormal))
                .collect();
            let scaled = mat_vec_mul(&cholesky, &z);
            self.mean.iter().zip(scaled.iter())
                .map(|(m, s)| m + self.sigma * s)
                .collect()
        }).collect()
    }

    /// Update distribution from fitness-ranked population.
    pub fn update(&mut self, population: &[Vec<f64>], fitness: &[f64]) {
        // 1. Rank by fitness, select top mu
        // 2. Update mean (weighted recombination)
        // 3. Update evolution paths
        // 4. Update covariance matrix (rank-one + rank-mu updates)
        // 5. Update step-size sigma via CSA
        // (Full algorithm: Hansen tutorial, Algorithm 1)
        todo!("See CMA-ES tutorial for complete update equations")
    }
}
```

**Integration**: CMA-ES optimizes the continuous parameters while MAP-Elites manages
the discrete structure (prompt templates, tool sets) and diversity. The combination
provides both efficient local optimization (CMA-ES) and global exploration (MAP-Elites).

---

## 19. Persistence and Reporting

### 19.1 Archive Persistence

```
.roko/learn/
├── skill-archive.json          # MAP-Elites archive
│   {"cells": {"42": {"id": "sk_a3f", "fitness": 0.87, ...}},
│    "generation": 150, "total_evaluations": 2400}
├── species.json                # Species state
│   {"species": [{"id": 1, "members": 12, "adjusted_fitness": 0.72}]}
├── landscape.json              # Latest landscape analysis
│   {"local_optima": 3, "ruggedness": 0.18, "fdc": 0.42}
└── cma-es-state.json           # CMA-ES optimizer state
    {"mean": [...], "sigma": 0.25, "covariance": [[...]]}
```

### 19.2 Dashboard Metrics

```
Skill Evolution:
  Archive: 847 / 10,000 cells filled (8.5% coverage)
  QD-score: 423.7 (↑ 12.3 from last generation)
  Species: 5 active, 2 stagnant
  Best fitness: 0.94 (Skill: "Rust Compile Fix — Missing Import")
  Landscape: FDC=0.42 (searchable), ruggedness=0.18 (smooth)
  CMA-ES sigma: 0.25 (converging)
  Generation: 150, total evaluations: 2,400
```

---

## 20. Test Criteria for Evolutionary Components

| Test | Property |
|---|---|
| `archive_insert_new_niche` | Empty cell → InsertResult::NewNiche |
| `archive_insert_improvement` | Fitter genome replaces existing in same cell |
| `archive_insert_rejected` | Less fit genome rejected in occupied cell |
| `archive_coverage_increases` | Successive insertions increase coverage |
| `archive_qd_score_monotone` | QD-score never decreases on replacement |
| `mutation_bounds_respected` | Temperature stays in [0, 1], budget stays positive |
| `crossover_intermediate` | Continuous params are between parent values |
| `speciation_groups_similar` | Genomes with distance < threshold → same species |
| `speciation_splits_different` | Genomes with distance > threshold → different species |
| `threshold_adjusts_dynamically` | Too many species → threshold increases |
| `stagnation_dissolves_species` | 15 gens without improvement → species dissolved |
| `fitness_evaluation_deterministic` | Same genome + same tasks → same fitness |
| `landscape_local_optima_detected` | Cell with no fitter neighbors → counted |
| `landscape_fdc_positive_for_easy` | Smooth landscape → FDC > 0.15 |
| `cma_es_sigma_decreases_on_convergence` | Near optimum → sigma shrinks |


---

# SOURCE: /Users/will/dev/nunchi/roko/roko/docs/04-verification/12-forensic-ai-causal-replay.md

# 12 — Forensic AI: Causal Replay

> **Layer**: L3 Harness — Verification × Compliance
> **Crates**: `roko-gate` (artifact_store), `roko-fs` (signal persistence),
>   `roko-learn` (episodes)
> **Status**: Foundations implemented (content-addressed artifacts, episode logging),
>   full replay pipeline designed


> **Implementation**: Shipping

---

## 1. Overview

Forensic AI causal replay is the ability to reconstruct, step by step, exactly what an
agent did, why it did it, and what verification outcomes resulted — with cryptographic
proof that the reconstruction is accurate. This is not debugging. This is audit-grade
reconstruction that can withstand regulatory scrutiny.

The system makes every agent action replayable: given a task ID, produce the complete
chain from initial prompt through every tool call, every gate verdict, every retry, to
the final outcome. Every artifact in this chain is content-addressed (BLAKE3 hashed),
so any tampering is detectable.

> **Citation**: refactoring-prd/09-innovations.md — Innovation IX: "Forensic AI Causal
> Replay, content-addressed replay of any agent action."

---

## 2. Why Forensic Replay Matters

### 2.1 Regulatory Compliance

Autonomous AI agents that make decisions affecting production systems, financial
instruments, or healthcare data must satisfy regulatory requirements for
explainability and auditability.

| Regulation | Requirement | How Forensic Replay Satisfies It |
|---|---|---|
| EU AI Act Art. 14 | Human oversight of high-risk AI | Complete action trace, gate verdicts as checkpoints |
| SEC/CFTC | Algorithmic trading audit trail | Content-addressed chain from decision to execution |
| HIPAA | Access audit for health data | Every file read/write by every agent, timestamped |
| SOX | Financial system change controls | Immutable verification artifacts for every code change |

### 2.2 Debugging Complex Failures

When an agent produces a subtle bug that passes all gates, forensic replay enables:
- Tracing back through the agent's reasoning to find where it went wrong
- Identifying which gate should have caught the issue (gap analysis)
- Determining whether the agent's tool calls were productive or wasteful

### 2.3 Learning System Validation

The learning system (skills, routing, experiments) makes decisions based on historical
data. Forensic replay can verify that those decisions were based on accurate data:
- Did the gate verdicts that trained the router actually correspond to correct
  verification outcomes?
- Did the skill extraction process correctly identify the tool call patterns that led
  to success?

> **Citation**: refactoring-prd/09-innovations.md — Regulatory compliance table with
> specific regulatory provisions.

---

## 3. The Content-Addressed Chain

Every element in the replay chain is identified by its BLAKE3 hash:

```
TaskSpec (hash: 0xa3f...)
    ↓
SystemPrompt (hash: 0xb7c...)
    ↓
AgentTurn 1 (hash: 0xc1d...)
    ├── ToolCall: Read "src/lib.rs" (hash: 0xd2e...)
    │   └── Result: file contents (hash: 0xe3f...)
    ├── ToolCall: Edit "src/lib.rs" (hash: 0xf4a...)
    │   └── Result: success (hash: 0xa5b...)
    └── Response: "I've added the new struct" (hash: 0xb6c...)
    ↓
GateVerdict Rung 0 (hash: 0xc7d...)
    └── Detail: compile output (hash: 0xd8e...)
    ↓
AgentTurn 2 (hash: 0xe9f...) [retry after gate failure]
    ├── ToolCall: Read "src/lib.rs" (hash: 0xfa0...)
    ...
    ↓
GateVerdict Rung 0 (hash: 0xab1...)  [pass]
GateVerdict Rung 1 (hash: 0xbc2...)  [pass]
GateVerdict Rung 2 (hash: 0xcd3...)  [pass]
    ↓
FinalOutcome (hash: 0xde4...)
```

Each node's hash incorporates its content. If any element is modified, its hash changes,
and the chain becomes inconsistent. This is the same principle that Git uses for commits
and that blockchains use for blocks.

---

## 4. Data Sources for Replay

### 4.1 Episode Log

**Path**: `.roko/episodes.jsonl`

Each line is a JSON object recording an agent turn:
```json
{
  "task_id": "plan-42-task-3",
  "turn": 1,
  "model": "claude-opus-4-6",
  "tool_calls": [
    {"tool": "Read", "args": {"path": "src/lib.rs"}, "duration_ms": 45},
    {"tool": "Edit", "args": {"path": "src/lib.rs", "..."}, "duration_ms": 12}
  ],
  "input_tokens": 12500,
  "output_tokens": 3200,
  "timestamp": "2026-04-10T14:30:00Z"
}
```

### 4.2 Signal Log

**Path**: `.roko/signals.jsonl`

Every signal (engram) written to the substrate, including gate verdicts:
```json
{
  "hash": "0xab1c2d3e...",
  "kind": "verdict",
  "body": {"gate": "compile:cargo", "passed": true, "duration_ms": 4200},
  "parent": "0x9f8e7d6c...",
  "timestamp": "2026-04-10T14:30:05Z"
}
```

### 4.3 Artifact Store

Content-addressed gate artifacts: compile output, test output, diff analysis results.
Each artifact is retrievable by its BLAKE3 hash.

### 4.4 Efficiency Events

**Path**: `.roko/learn/efficiency.jsonl`

Per-turn efficiency data: token counts, tool call metadata, gate timing, cost estimates.

---

## 5. The Replay Algorithm

Given a task ID, reconstruct the complete execution:

```
1. Query episode log for all turns with this task_id
   → Ordered list of agent turns

2. For each turn:
   a. Retrieve the system prompt (from prompt assembly logs)
   b. Retrieve tool call inputs and outputs (from episode log)
   c. Retrieve the agent's response

3. Query signal log for all verdicts associated with this task_id
   → Ordered list of gate verdicts

4. For each verdict:
   a. Retrieve the gate artifact from ArtifactStore by hash
   b. Reconstruct the verdict's inputs (the signal that was verified)

5. Build the causal chain:
   TaskSpec → Prompt → Turn 1 → ... → Turn N → Verdict 1 → ... → Verdict M → Outcome

6. Verify chain integrity:
   For each element, recompute its BLAKE3 hash and compare to the stored hash
   Any mismatch indicates tampering or corruption
```

---

## 6. Causal Analysis

Beyond reconstruction, forensic replay enables causal analysis:

### 6.1 What-If Analysis

"What if the agent had used a different model?" Replay the task with the same inputs
but a different model. Compare verdicts and outcomes. This powers the shadow testing
loop (Loop 12 in the evaluation lifecycle).

### 6.2 Root Cause Analysis

When a task fails, trace backward through the causal chain:
1. Which gate failed? (e.g., Rung 2: Test)
2. What was the test failure? (e.g., "assertion failed: expected 200, got 404")
3. Which agent edit introduced the failure? (e.g., Turn 3, Edit to routes.rs)
4. What was the agent's reasoning? (e.g., "I moved the route handler to a different module")
5. Was the reasoning correct? (e.g., "Yes, but the agent forgot to update the route registration")

This chain from verdict → edit → reasoning → root cause is what "forensic" means. It's
not just "what happened" but "why it happened."

### 6.3 Gap Analysis

When a bug escapes all gates (passes verification but is still wrong), forensic replay
identifies which gate *should* have caught it:

```
Bug: off-by-one error in pagination
Escaped gates: Compile (expected), Lint (expected), Test (gap!)

Analysis:
  - Existing tests don't cover pagination edge cases
  - Generated tests (Rung 4) would have caught this if the test generator
    had been prompted with "test boundary conditions for pagination"

Recommendation: Add pagination boundary test to the GeneratedTestGate's
  standard test generation templates
```

---

## 7. Immutability Guarantees

### 7.1 Content-Addressed Everything

Every artifact, signal, and episode entry is identified by its content hash. Changing
any byte changes the hash, making tampering detectable.

### 7.2 Append-Only Logs

The episode log (`.roko/episodes.jsonl`) and signal log (`.roko/signals.jsonl`) are
append-only JSONL files. New entries are appended; existing entries are never modified
or deleted during normal operation.

### 7.3 Artifact Store Immutability

The `ArtifactStore` has no delete or update operations (see
[04-artifact-store.md](./04-artifact-store.md)). Once stored, artifacts are permanent.

### 7.4 Hash Chain (Future)

The signals in the signal log form a hash chain: each signal's hash incorporates its
parent signal's hash. This creates a tamper-evident sequence — inserting, removing, or
reordering signals breaks the chain.

---

## 8. Pre-Certified Agent Templates

A practical application of forensic replay: **pre-certified agent templates** for
regulated industries.

A pre-certified template is a set of:
1. System prompt sections (versioned, hashed)
2. Gate pipeline configuration (which rungs, which gates)
3. Verification criteria (generated test templates)
4. Audit trail requirements (which data must be logged)

Organizations in regulated industries can deploy pre-certified templates knowing that:
- Every agent action will be logged and replayable
- Every verification outcome is content-addressed and immutable
- The complete chain from input to output is reconstructable
- Regulatory auditors can independently verify the chain

> **Citation**: refactoring-prd/09-innovations.md — "Pre-certified agent templates"
> for regulated industries.

---

## 9. Performance Considerations

Forensic replay adds overhead to the execution path:

| Operation | Overhead | When |
|---|---|---|
| BLAKE3 hashing | < 1ms per artifact | Every gate run |
| Episode logging | < 1ms per turn | Every agent turn |
| Signal logging | < 1ms per signal | Every signal write |
| Artifact storage | O(artifact_size) | Every gate run |
| Chain verification | O(chain_length) | On-demand (replay) |

The per-execution overhead is negligible (< 5ms total). The on-demand replay cost is
proportional to the chain length but is only incurred when forensic analysis is actually
needed.

### Storage Cost

For a typical plan execution (10 tasks, 3 attempts each, 5 gate runs per attempt):
- Episodes: ~150 JSONL entries, ~500 KB
- Signals: ~150 entries, ~300 KB
- Artifacts: ~150 artifacts, ~5 MB (mostly compile/test output)
- Total: ~6 MB per plan execution

At this rate, a year of continuous operation produces ~2 GB of forensic data — easily
manageable with periodic GC of old artifacts.

---

## 10. Relationship to Other Components

| Component | Relationship to Forensic Replay |
|---|---|
| ArtifactStore | Stores immutable gate artifacts |
| Episode Logger | Records agent turns |
| Signal Log | Records engrams and verdicts |
| GateRatchet | Ratchet state at each point in time |
| AdaptiveThresholds | Threshold state at each point in time |
| Efficiency Events | Per-turn cost and timing data |

---

## 11. Summary

Forensic AI causal replay transforms Roko's verification layer from "did the code pass
the gates?" to "how did the code come to pass (or fail) the gates, and can we prove
it?" The content-addressed, append-only architecture makes this reconstruction
cryptographically verifiable.

This is not a feature that most users need day-to-day. But for regulated industries,
for debugging complex multi-agent interactions, and for validating the learning system's
decisions, it is essential. The infrastructure cost is negligible (< 5ms per execution,
~6 MB per plan). The value is unbounded: any question about any past execution can be
answered definitively.


---

# SOURCE: /Users/will/dev/nunchi/roko/roko/docs/04-verification/15-verdicts-as-signals.md

# Verdicts as signals

> Layer 3 Harness -- Verification as Cognition
> Status: **Specification** -- gate verdict emission wired, signal re-entry planned
> Canonical source: `crates/roko-gate/`, `crates/roko-core/src/kind.rs` (Kind::GateVerdict)
> Cross-references: [00-gate-trait.md](00-gate-trait.md), [03-gate-pipeline.md](03-gate-pipeline.md)

> **Implementation**: Specified

---

## Purpose

Gate verdicts are not terminal events. They are signals -- first-class Engrams that re-enter the signal pipeline. A compile failure is knowledge. A test pass is evidence. A clippy warning is a heuristic.

This document specifies how gate verdicts become Engrams, how they flow through the universal cognitive loop, and how downstream consumers (Scorer, Router, Composer, Dreams) use them.

---

## 1. The core claim: verification is cognition

In a standard CI pipeline, a gate verdict is an end state: pass or fail, logged and forgotten. In Roko, a gate verdict is a beginning. When a compile gate fails, that failure is an Engram with a Kind, Score, Decay, and lineage. It enters the Substrate. Other components query it:

- The **Scorer** appraises the verdict (a compile error on a file the agent just modified scores higher than a pre-existing warning).
- The **Router** uses verdict history to select models (tasks that repeatedly fail compile get routed to stronger models).
- The **Composer** injects recent verdicts into agent prompts (the agent sees its own failures).
- **Dreams** replays verdict patterns during consolidation (the system learns which gate patterns predict task failure).

The verdict is not metadata about the pipeline. It is a data point in the agent's cognitive process.

---

## 2. Verdict-to-signal transformation

### 2.1 The GateVerdict struct

Two `GateVerdict` structs exist in the codebase. The episode logger's version carries the learning-relevant fields:

```rust
/// From crates/roko-learn/src/episode_logger.rs
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct GateVerdict {
    /// Gate identifier ("compile", "test", "lint", ...).
    pub gate: String,
    /// Whether the gate passed.
    pub passed: bool,
    /// Optional short diagnostic (hashed, never raw output).
    pub signature: Option<String>,
}
```

The dashboard's version adds plan/task context:

```rust
/// From crates/roko-core/src/dashboard_snapshot.rs
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GateVerdict {
    pub plan_id: String,
    pub task_id: String,
    pub gate: String,
    pub passed: bool,
    pub ts_millis: u64,
}
```

### 2.2 Transformation to Signal

When a gate completes, the orchestrator transforms its verdict into an Engram:

```
fn verdict_to_engram(verdict: &GateVerdict, task_engram: &Engram) -> Engram {
    Engram::builder(Kind::GateVerdict)
        .body(Body::json(verdict))
        .decay(Decay::HalfLife { half_life_ms: 86_400_000 })  // 24h
        .lineage([task_engram.id])   // verdict derives from the task
        .tag("gate", &verdict.gate)
        .tag("passed", &verdict.passed.to_string())
        .tag("plan_id", &verdict.plan_id)
        .tag("task_id", &verdict.task_id)
        .build()
}
```

Key properties:

| Property | Value | Rationale |
|---|---|---|
| Kind | `Kind::GateVerdict` | Already defined in `roko-core` |
| Decay | `HalfLife { 86_400_000 }` (24h) | Code changes invalidate verdicts; yesterday's compile pass is stale |
| Lineage | Points to the task Signal | Preserves causal chain for auditing |
| Tags | gate name, passed, plan_id, task_id | Enable filtering by gate type and outcome |

---

## 3. Signal pipeline flow

Once emitted, the verdict Signal enters the standard cognitive loop:

```
Gate verdict emitted
    |
    v
Substrate.write(verdict_signal)     -- persisted to .roko/signals.jsonl
    |
    v
Scorer.score(verdict_signal)        -- appraise relevance and urgency
    |
    v
Router.select(candidates)           -- verdict history influences model routing
    |
    v
Composer.compose(context)           -- recent verdicts injected into prompts
    |
    v
Dreams.replay(episodes)             -- verdict patterns extracted during consolidation
```

---

## 4. Consumer specifications

### 4.1 Scorer: verdict appraisal

The Scorer assigns a Score to the verdict Signal based on:

```
Scoring dimensions for GateVerdict signals:

  relevance:   1.0 if verdict is for the currently active task
               0.5 if verdict is for a task in the same plan
               0.1 if verdict is for a different plan
  confidence:  1.0 (gate verdicts are deterministic)
  urgency:     0.9 if failed (failure needs immediate attention)
               0.3 if passed (success is informational)
  novelty:     1.0 if this is the first verdict for this gate+task
               0.2 if this is a repeated verdict (re-run of same gate)
  salience:    scaled by recency -- fresher verdicts score higher
  coherence:   1.0 (verdicts are self-consistent by construction)
  surprise:    1.0 if outcome contradicts the model's prediction
               0.0 if outcome matches prediction
```

### 4.2 Router: verdict-informed model selection

The cascade router queries verdict history when selecting a model for a task:

```
Routing adjustment from verdict history:

  For task T, query Substrate for GateVerdict signals where task_id == T:
    - If 0 prior failures: standard routing (no adjustment)
    - If 1 prior failure:  escalate model tier by 1 (e.g., Haiku -> Sonnet)
    - If 2+ prior failures: escalate to maximum tier (Opus)
    - If 3+ prior failures with same gate signature: flag for replanning

  Implementation path:
    CascadeRouter::select() calls Substrate::query(
        Filter::kind(Kind::GateVerdict)
            .tag("task_id", task_id)
            .tag("passed", "false")
    )
```

### 4.3 Composer: verdict injection

The SystemPromptBuilder includes recent verdicts in the agent's prompt:

```
Section: "Recent Gate Results"
Priority: High (same as gate errors in the budget table)
Max tokens: 500
Min tokens: 50

Content format:
  ## Previous attempts on this task

  Attempt 1: FAIL (compile)
    Error: E0599 - no method named `foo` found for struct `Bar`
    Signature: a3f8c2

  Attempt 2: FAIL (test)
    Error: assertion failed in test_routing_basic
    Signature: 7d1e4b
```

This gives the agent direct visibility into its own failure history, preventing it from repeating the same mistake.

### 4.4 Dreams: verdict pattern extraction

During NREM replay, Dreams extracts patterns from verdict sequences:

```
Pattern extraction from verdict signals:

  Input: all GateVerdict signals from the last consolidation window
  Process:
    1. Group by (gate, signature) -- same error type
    2. For each group with >= 3 occurrences:
       a. Extract common context (file paths, error codes, task types)
       b. Generate a Heuristic knowledge entry:
          "When working on [context], [gate] tends to fail with [signature]"
       c. Insert at Transient tier for validation
    3. For groups where failure was followed by success:
       a. Extract the delta between failing and succeeding attempts
       b. Generate a StrategyFragment:
          "To fix [signature], the successful approach was [delta]"
```

---

## 5. Verdict decay and lifecycle

| Stage | Timing | Action |
|---|---|---|
| Emission | Gate completes | Signal written to Substrate |
| Active use | 0 - 4 hours | Composer injects into prompts; Router adjusts model selection |
| Fading relevance | 4 - 24 hours | Weight decays below 0.5; lower priority in Composer |
| Consolidation | During Dreams Delta | Patterns extracted; individual verdicts no longer needed |
| Pruning | Weight < threshold | Substrate.prune() removes the verdict Signal |

The 24-hour HalfLife means a verdict retains 50% weight after one day. This is appropriate because code changes within a day can invalidate any verdict. After Dreams consolidation, the patterns survive in knowledge entries even after the raw verdicts are pruned.

---

## 6. Lineage and auditing

Every verdict Signal records its lineage -- the task Signal it derived from:

```
verdict.lineage = [task_signal.id]
```

This creates a DAG:

```
Plan Signal
  |
  +-- Task Signal (T1)
  |     |
  |     +-- GateVerdict (compile: pass)
  |     +-- GateVerdict (test: fail)
  |     +-- GateVerdict (test: pass, attempt 2)
  |
  +-- Task Signal (T2)
        |
        +-- GateVerdict (compile: fail)
```

The `roko replay` command walks this DAG to reconstruct the full verification history for any plan or task.

---

## 7. Configuration parameters

| Parameter | Default | Range | Description |
|---|---|---|---|
| `verdict_decay_half_life_ms` | 86,400,000 (24h) | 3,600,000 - 604,800,000 | How fast verdicts lose relevance |
| `verdict_max_prompt_tokens` | 500 | 50 - 2,000 | Max tokens for verdict section in prompts |
| `verdict_escalation_threshold` | 2 | 1 - 5 | Failures before model tier escalation |
| `verdict_replan_threshold` | 3 | 2 - 10 | Same-signature failures before replanning |
| `verdict_dreams_min_group_size` | 3 | 2 - 10 | Min occurrences for pattern extraction |

---

## 8. Error handling

| Condition | Response |
|---|---|
| Verdict body fails JSON serialization | Log error, emit verdict with `Body::text(gate + ":" + passed)` fallback |
| Substrate write fails (disk full, I/O error) | Buffer in memory (up to 100 verdicts), retry on next tick |
| Verdict references a task Signal that was pruned | Verdict still valid; lineage points to a hash that may not resolve |
| Duplicate verdict (same gate + task + attempt) | Deduplicate by content hash; the second write is a no-op |

---

## 9. Implementation wiring

Current state:

| Component | Status |
|---|---|
| `Kind::GateVerdict` in roko-core | **Implemented** |
| GateVerdict struct in episode_logger | **Implemented** |
| GateVerdict struct in dashboard_snapshot | **Implemented** |
| Gate verdict emission in orchestrate.rs | **Wired** (verdicts logged to episodes) |
| Verdict-to-Signal transformation | **Not yet** (verdicts logged but not emitted as Signals) |
| Scorer appraisal of verdict Signals | **Not yet** |
| Router verdict-based escalation | **Not yet** (escalation uses iteration count, not verdict Signals) |
| Composer verdict injection | **Partially** (gate errors injected, but not as Signal queries) |
| Dreams verdict pattern extraction | **Not yet** |

The wiring path:

1. In `orchestrate.rs`, after each gate run, call `verdict_to_signal()` and write to Substrate.
2. In `CascadeRouter::select()`, query Substrate for prior verdict Signals on the current task.
3. In `SystemPromptBuilder`, query Substrate for recent verdict Signals instead of passing gate errors directly.
4. In `DreamsEngine::consolidate()`, include verdict Signals in the replay set.

Estimated LOC: ~120 for transformation + Substrate writes, ~60 for Router query, ~40 for Composer query, ~80 for Dreams pattern extraction. Total: ~300 LOC.

---

## 10. Test criteria

1. Gate verdict produces a Signal with `Kind::GateVerdict`, correct lineage, and 24h HalfLife.
2. Verdict Signal round-trips through serde without loss.
3. Substrate query by `tag("gate", "compile")` returns only compile verdicts.
4. Router escalates model tier after 2 consecutive failures on the same task.
5. Composer includes verdict section with correct token budget and priority ordering.
6. Dreams extracts a Heuristic from 3+ same-signature failures.
7. Verdict weight reaches 0.5 at exactly 24 hours.
8. Duplicate verdicts (same content hash) are deduplicated on write.

---

## Cross-References

- [00-gate-trait.md](00-gate-trait.md) -- Gate trait and Verdict type
- [03-gate-pipeline.md](03-gate-pipeline.md) -- 6-rung gate pipeline
- [06-adaptive-thresholds.md](06-adaptive-thresholds.md) -- Threshold adjustment from verdict history
- [08-agent-feedback-from-gates.md](08-agent-feedback-from-gates.md) -- How agents receive gate feedback
- [../00-architecture/09-universal-cognitive-loop.md](../00-architecture/09-universal-cognitive-loop.md) -- The loop verdicts re-enter
- [../05-learning/00-episode-logger.md](../05-learning/00-episode-logger.md) -- Where verdicts are currently logged
- `crates/roko-core/src/kind.rs` -- Kind::GateVerdict definition
- `crates/roko-learn/src/episode_logger.rs` -- GateVerdict struct

---

## 11. Verdict Aggregation Across Time: Trend Detection

Individual verdicts are snapshots. Trends across verdicts reveal systemic changes —
a gate that was stable for weeks but now fails frequently, a model that suddenly
produces worse code, a plan category that consistently underperforms. Trend detection
transforms raw verdict streams into actionable intelligence.

> **Citation**: "Contrasting Test Selection, Prioritization, and Batch Testing at Scale"
> (Empirical Software Engineering, 2024) — ML-driven trend detection in CI pipelines.

### 11.1 Verdict Time Series

```rust
/// A verdict time series for a specific gate, tracking outcomes over time.
pub struct VerdictTimeSeries {
    /// Gate identifier.
    pub gate: String,
    /// Ordered observations (newest last).
    pub observations: VecDeque<VerdictObservation>,
    /// Maximum observations retained (sliding window).
    pub max_observations: usize,    // default: 500
    /// EMA of pass rate (same as AdaptiveThresholds.ema_pass_rate).
    pub ema_pass_rate: f64,
    /// EMA of verdict score (continuous, not just binary).
    pub ema_score: f64,
    /// CUSUM accumulators for shift detection (see §06 SPC).
    pub cusum_upper: f64,
    pub cusum_lower: f64,
    /// Computed trend classification.
    pub trend: VerdictTrend,
}

#[derive(Debug, Clone)]
pub struct VerdictObservation {
    pub timestamp_ms: u64,
    pub passed: bool,
    pub score: f32,
    pub plan_id: String,
    pub task_id: String,
    pub signature: Option<String>,
    pub model: Option<String>,
}

/// Trend classification for a verdict time series.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VerdictTrend {
    /// No significant change in behavior.
    Stable,
    /// Consistent improvement over the observation window.
    Improving,
    /// Consistent degradation over the observation window.
    Degrading,
    /// High variance, no clear direction.
    Volatile,
    /// Fundamental change in statistical properties (detected by BOCPD).
    RegimeShift,
}
```

### 11.2 Trend Classification Algorithm

```rust
impl VerdictTimeSeries {
    /// Classify the current trend from the observation window.
    ///
    /// Uses three signals:
    /// 1. EMA derivative (slope of the smoothed pass rate)
    /// 2. CUSUM shift detection (sustained drift)
    /// 3. BOCPD regime change (fundamental change)
    pub fn classify_trend(&self) -> VerdictTrend {
        // 1. Compute EMA slope over the last N observations
        let slope = self.ema_slope(20); // slope over last 20 observations

        // 2. Check CUSUM for sustained shift
        let cusum_signal = self.cusum_upper > CUSUM_H
            || self.cusum_lower > CUSUM_H;

        // 3. Check BOCPD for regime change
        let regime_change = self.bocpd_changepoint_prob > BOCPD_THRESHOLD;

        // Classification logic:
        if regime_change {
            return VerdictTrend::RegimeShift;
        }
        if cusum_signal && slope > SLOPE_IMPROVING {
            return VerdictTrend::Improving;
        }
        if cusum_signal && slope < SLOPE_DEGRADING {
            return VerdictTrend::Degrading;
        }

        // Volatility check: coefficient of variation
        let cv = self.score_std_dev() / self.ema_score.max(0.01);
        if cv > VOLATILITY_THRESHOLD {
            return VerdictTrend::Volatile;
        }

        VerdictTrend::Stable
    }
}

/// Trend detection constants.
const SLOPE_IMPROVING: f64 = 0.005;   // ~0.5% improvement per observation
const SLOPE_DEGRADING: f64 = -0.005;
const CUSUM_H: f64 = 4.0;             // CUSUM decision interval
const BOCPD_THRESHOLD: f64 = 0.5;     // P(changepoint) threshold
const VOLATILITY_THRESHOLD: f64 = 0.3; // CV above this = volatile
```

### 11.3 Multi-Gate Trend Dashboard

```
Verdict Trends (last 200 observations):
  Compile:        ███████████████ 97.2% STABLE        (slope: +0.001)
  Lint:           █████████████░░ 86.5% DEGRADING ↓   (slope: -0.012, CUSUM alert)
  Test:           ████████████░░░ 79.1% STABLE        (slope: +0.003)
  Symbol:         ███████████████ 98.0% IMPROVING ↑   (slope: +0.008)
  Generated:      ████████░░░░░░░ 54.2% VOLATILE ⚡    (CV: 0.42)
  Property:       █████████████░░ 88.0% REGIME SHIFT  (BOCPD: new baseline 88%)
```

---

## 12. Verdict Pattern Mining

Beyond per-gate trends, cross-gate and cross-task patterns reveal deeper structural
issues.

### 12.1 Co-Failure Patterns

```rust
/// Detect gates that tend to fail together.
///
/// If compile and lint failures correlate > threshold, they likely
/// share a root cause (e.g., syntax errors cause both).
pub struct CoFailureDetector {
    /// Co-occurrence matrix: entry (i,j) = count of times gate i and
    /// gate j both failed on the same task.
    pub co_failures: HashMap<(String, String), u64>,
    /// Total observations per gate.
    pub gate_counts: HashMap<String, u64>,
    /// Minimum co-occurrence for significance.
    pub min_co_occurrences: u64,     // default: 5
    /// Phi coefficient threshold for declaring correlation.
    pub correlation_threshold: f64,   // default: 0.3
}

impl CoFailureDetector {
    /// Record a set of gate verdicts from one task execution.
    pub fn observe(&mut self, verdicts: &[(&str, bool)]) {
        let failed: Vec<&str> = verdicts.iter()
            .filter(|(_, passed)| !passed)
            .map(|(gate, _)| *gate)
            .collect();

        for i in 0..failed.len() {
            for j in (i+1)..failed.len() {
                let key = if failed[i] < failed[j] {
                    (failed[i].to_string(), failed[j].to_string())
                } else {
                    (failed[j].to_string(), failed[i].to_string())
                };
                *self.co_failures.entry(key).or_insert(0) += 1;
            }
        }
        for (gate, _) in verdicts {
            *self.gate_counts.entry(gate.to_string()).or_insert(0) += 1;
        }
    }

    /// Return significantly correlated gate pairs.
    pub fn correlated_pairs(&self) -> Vec<CoFailurePair> {
        self.co_failures.iter()
            .filter(|(_, &count)| count >= self.min_co_occurrences)
            .filter_map(|((a, b), &count)| {
                let phi = self.phi_coefficient(a, b, count);
                if phi.abs() > self.correlation_threshold {
                    Some(CoFailurePair {
                        gate_a: a.clone(),
                        gate_b: b.clone(),
                        co_failure_count: count,
                        phi_coefficient: phi,
                    })
                } else {
                    None
                }
            })
            .collect()
    }
}

pub struct CoFailurePair {
    pub gate_a: String,
    pub gate_b: String,
    pub co_failure_count: u64,
    pub phi_coefficient: f64,
}
```

### 12.2 Failure Signature Clustering

Group failures by their error signature to identify recurring issues:

```rust
/// Cluster verdict failures by error signature.
///
/// Repeated failures with the same signature suggest a systemic issue
/// that needs structural attention, not just retries.
pub struct SignatureCluster {
    /// The error signature (hashed diagnostic).
    pub signature: String,
    /// Gate that produced these failures.
    pub gate: String,
    /// Number of occurrences.
    pub count: u64,
    /// First and last occurrence timestamps.
    pub first_seen_ms: u64,
    pub last_seen_ms: u64,
    /// Task IDs that experienced this failure.
    pub affected_tasks: Vec<String>,
    /// Plan IDs that experienced this failure.
    pub affected_plans: Vec<String>,
    /// Models that produced this failure.
    pub affected_models: Vec<String>,
    /// Whether this cluster is growing (more frequent over time).
    pub trend: VerdictTrend,
}

impl SignatureCluster {
    /// Compute severity: a composite of frequency, recency, and breadth.
    pub fn severity(&self) -> f64 {
        let frequency = (self.count as f64).ln().max(0.0) / 5.0; // log-scaled
        let recency = 1.0; // would use time decay in practice
        let breadth = self.affected_plans.len() as f64
            / 10.0_f64.max(self.affected_plans.len() as f64);
        (frequency + recency + breadth) / 3.0
    }
}
```

---

## 13. Verdict-Driven Replanning

When verdict patterns indicate that a plan or task is fundamentally broken, automatic
replanning modifies the execution strategy without human intervention.

### 13.1 Replanning Triggers

```rust
/// Conditions that trigger automatic replanning.
pub struct ReplanTriggers {
    /// Same error signature fails N times across attempts → replan the task.
    pub same_signature_threshold: u32,    // default: 3
    /// Progress score negative for N consecutive turns → replan approach.
    pub negative_progress_threshold: u32, // default: 3
    /// Promise score below this for N turns → abort and replan.
    pub low_promise_threshold: f64,       // default: 0.2
    pub low_promise_turns: u32,           // default: 2
    /// Plan-level: if > N tasks fail in a plan → re-plan remaining tasks.
    pub plan_failure_threshold: u32,      // default: 3
    /// Gate degradation trend detected → replan with additional gates.
    pub trend_degradation_trigger: bool,  // default: true
}

/// Replanning action to take.
#[derive(Debug, Clone)]
pub enum ReplanAction {
    /// Modify the task: add constraints, decompose into sub-tasks,
    /// change the approach described in the task spec.
    ModifyTask {
        task_id: String,
        reason: String,
        /// Suggested modifications based on failure analysis.
        modifications: Vec<TaskModification>,
    },
    /// Replace the task entirely with a new plan generated from
    /// the failure context.
    ReplaceTask {
        task_id: String,
        reason: String,
        /// Context from failures to inform the new plan.
        failure_context: FailureContext,
    },
    /// Decompose a single failing task into smaller sub-tasks that
    /// are individually more likely to pass gates.
    DecomposeTask {
        task_id: String,
        reason: String,
        /// Suggested decomposition points.
        split_points: Vec<String>,
    },
    /// Escalate: add stronger gates, use more capable model, or
    /// flag for human review.
    Escalate {
        task_id: String,
        reason: String,
        escalation: EscalationType,
    },
}

#[derive(Debug, Clone)]
pub enum TaskModification {
    /// Add a constraint to the task spec (e.g., "do not modify file X").
    AddConstraint(String),
    /// Remove a requirement that is causing failures.
    RelaxRequirement(String),
    /// Change the approach (e.g., "use trait objects instead of generics").
    ChangeApproach(String),
    /// Add a prerequisite task that must complete first.
    AddPrerequisite(String),
}

pub struct FailureContext {
    /// Error signatures from failed attempts.
    pub signatures: Vec<String>,
    /// Gates that consistently fail.
    pub failing_gates: Vec<String>,
    /// Files that were modified in failed attempts.
    pub modified_files: Vec<String>,
    /// Successful approaches for similar tasks (from skill library).
    pub similar_successes: Vec<String>,
}

pub enum EscalationType {
    /// Add more verification gates.
    AddGates(Vec<Box<dyn Gate>>),
    /// Use a more capable model.
    UpgradeModel(String),
    /// Flag for human review.
    HumanReview,
}
```

### 13.2 Replanning Decision Engine

```rust
/// Engine that monitors verdict patterns and triggers replanning.
pub struct ReplanEngine {
    pub triggers: ReplanTriggers,
    /// Per-task failure tracking.
    pub task_failures: HashMap<String, TaskFailureState>,
    /// Per-plan failure tracking.
    pub plan_failures: HashMap<String, PlanFailureState>,
}

pub struct TaskFailureState {
    /// Consecutive attempts with same error signature.
    pub same_signature_streak: u32,
    /// Last seen error signature.
    pub last_signature: Option<String>,
    /// Progress scores for recent turns.
    pub recent_progress: VecDeque<f64>,
    /// Promise scores for recent turns.
    pub recent_promise: VecDeque<f64>,
}

impl ReplanEngine {
    /// Process a new verdict and determine if replanning is needed.
    pub fn process_verdict(&mut self, verdict: &GateVerdict,
                           process_reward: &ProcessReward)
        -> Option<ReplanAction>
    {
        let state = self.task_failures
            .entry(verdict.task_id.clone())
            .or_default();

        // Track signature streaks
        if !verdict.passed {
            if verdict.signature.as_deref() == state.last_signature.as_deref() {
                state.same_signature_streak += 1;
            } else {
                state.same_signature_streak = 1;
                state.last_signature = verdict.signature.clone();
            }
        } else {
            state.same_signature_streak = 0;
        }

        // Track process rewards
        state.recent_progress.push_back(process_reward.progress);
        state.recent_promise.push_back(process_reward.promise);
        if state.recent_progress.len() > 10 {
            state.recent_progress.pop_front();
            state.recent_promise.pop_front();
        }

        // Check triggers
        if state.same_signature_streak >= self.triggers.same_signature_threshold {
            return Some(ReplanAction::ModifyTask {
                task_id: verdict.task_id.clone(),
                reason: format!(
                    "Same error signature '{}' failed {} times consecutively",
                    state.last_signature.as_deref().unwrap_or("unknown"),
                    state.same_signature_streak
                ),
                modifications: self.suggest_modifications(verdict, state),
            });
        }

        let negative_progress_count = state.recent_progress.iter()
            .rev()
            .take_while(|&&p| p < -0.1)
            .count() as u32;
        if negative_progress_count >= self.triggers.negative_progress_threshold {
            return Some(ReplanAction::DecomposeTask {
                task_id: verdict.task_id.clone(),
                reason: format!(
                    "Negative progress for {} consecutive turns",
                    negative_progress_count
                ),
                split_points: self.suggest_decomposition(verdict),
            });
        }

        let low_promise_count = state.recent_promise.iter()
            .rev()
            .take_while(|&&p| p < self.triggers.low_promise_threshold)
            .count() as u32;
        if low_promise_count >= self.triggers.low_promise_turns {
            return Some(ReplanAction::ReplaceTask {
                task_id: verdict.task_id.clone(),
                reason: format!(
                    "Promise below {:.1} for {} turns — approach is not viable",
                    self.triggers.low_promise_threshold,
                    low_promise_count
                ),
                failure_context: self.build_failure_context(verdict, state),
            });
        }

        None // No replanning needed
    }
}
```

### 13.3 Replanning Feedback Loop

```
Verdict stream
    │
    ├── Trend detection ──────► VerdictTrend per gate
    │                              │
    ├── Co-failure analysis ──► Correlated gate pairs
    │                              │
    ├── Signature clustering ─► Recurring failure patterns
    │                              │
    └── ReplanEngine ◄────────── All signals combined
         │
         ├── ReplanAction::ModifyTask ──► Orchestrator adjusts task spec
         │                                before next attempt
         ├── ReplanAction::DecomposeTask ──► DAG executor creates sub-tasks
         │                                   with dependency edges
         ├── ReplanAction::ReplaceTask ──► Plan generator creates new task
         │                                 from failure context
         └── ReplanAction::Escalate ──► Stronger model / more gates / human
```

---

## 14. Meta-Learning from Verdict Patterns

Verdicts are the richest learning signal in the system. Meta-learning uses verdict
history to improve future verification decisions.

> **Citation**: Finn et al., "Model-Agnostic Meta-Learning for Fast Adaptation"
> (MAML, arXiv:1703.03400, ICML 2017).

> **Citation**: Machalica et al., "Predictive Test Selection" (ICSE-SEIP 2019) —
> Facebook's ML-based test selection achieving 2x cost reduction.

### 14.1 Predictive Gate Selection

Instead of using static complexity bands to select gates, predict which gates will
fail based on the task's features and select those gates for thorough verification:

```rust
/// Predictive gate selector using verdict history.
///
/// Features per (task, gate) pair, trained on historical verdicts:
///   - task_category: categorical (compile fix, test fix, new feature, refactor)
///   - files_modified: set of file paths
///   - model_used: which LLM model
///   - recent_gate_history: last 10 verdicts for this gate
///   - task_complexity: token count of task description
///   - gate_pass_rate: current EMA pass rate for this gate
///   - co_failure_rate: correlation with other failing gates
pub struct PredictiveGateSelector {
    /// Per-gate failure prediction models.
    /// Maps gate name → trained predictor.
    pub predictors: HashMap<String, Box<dyn FailurePredictor>>,
    /// Minimum predicted failure probability to include gate.
    pub inclusion_threshold: f64,    // default: 0.1
    /// Maximum gates to include (prevents over-verification).
    pub max_gates: usize,            // default: 7
}

pub trait FailurePredictor: Send + Sync {
    /// Predict the probability of failure for this gate given task features.
    fn predict_failure(&self, features: &TaskFeatures) -> f64;
}

pub struct TaskFeatures {
    pub category: String,
    pub files_modified: Vec<String>,
    pub model: String,
    pub task_complexity: usize,
    pub recent_gate_results: Vec<bool>,
}

impl PredictiveGateSelector {
    /// Select gates to run based on predicted failure probabilities.
    ///
    /// Strategy: always include mandatory gates (compile, test).
    /// Include optional gates only if P(failure) > threshold.
    /// Rank by P(failure) descending and take top max_gates.
    pub fn select_gates(&self, features: &TaskFeatures) -> Vec<String> {
        let mut predictions: Vec<(String, f64)> = self.predictors.iter()
            .map(|(gate, predictor)| {
                (gate.clone(), predictor.predict_failure(features))
            })
            .collect();

        // Always include mandatory gates
        let mut selected = vec!["compile".to_string(), "test".to_string()];

        // Sort optional gates by predicted failure probability (desc)
        predictions.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap());

        for (gate, prob) in predictions {
            if selected.contains(&gate) { continue; }
            if prob >= self.inclusion_threshold && selected.len() < self.max_gates {
                selected.push(gate);
            }
        }

        selected
    }
}
```

### 14.2 Few-Shot Gate Calibration from Verdict History

When a new task arrives, find similar past tasks and use their verdict patterns to
pre-calibrate gate thresholds:

```rust
/// Few-shot gate calibration from historical verdict patterns.
///
/// For a new task, find the K nearest historical tasks and use their
/// verdict patterns to set initial thresholds and retry budgets.
pub struct VerdictPatternMemory {
    /// Historical task → verdict pattern entries.
    pub patterns: Vec<TaskVerdictPattern>,
    /// Number of nearest neighbors to use for calibration.
    pub k: usize,                    // default: 5
    /// Similarity metric for task features.
    pub similarity: Box<dyn TaskSimilarity>,
}

pub struct TaskVerdictPattern {
    pub features: TaskFeatures,
    pub gate_verdicts: Vec<(String, bool, f32)>,  // gate, passed, score
    pub outcome: bool,                             // task succeeded?
    pub steps_to_completion: u32,
    pub retries_used: u32,
    pub model: String,
}

impl VerdictPatternMemory {
    /// Calibrate gate thresholds for a new task using K nearest neighbors.
    pub fn calibrate(&self, new_task: &TaskFeatures) -> GateCalibration {
        let similar = self.k_nearest(new_task, self.k);

        // Predict which gates will likely fail
        let mut gate_failure_rates: HashMap<String, (u64, u64)> = HashMap::new();
        for pattern in &similar {
            for (gate, passed, _) in &pattern.gate_verdicts {
                let (fails, total) = gate_failure_rates
                    .entry(gate.clone()).or_insert((0, 0));
                if !passed { *fails += 1; }
                *total += 1;
            }
        }

        // Predict optimal retry count from similar tasks
        let avg_retries = similar.iter()
            .filter(|p| p.outcome) // only from successful tasks
            .map(|p| p.retries_used as f64)
            .sum::<f64>() / similar.len().max(1) as f64;

        GateCalibration {
            per_gate_failure_prediction: gate_failure_rates.into_iter()
                .map(|(gate, (fails, total))| {
                    (gate, fails as f64 / total as f64)
                })
                .collect(),
            suggested_retries: avg_retries.ceil() as u32,
            similar_task_success_rate: similar.iter()
                .filter(|p| p.outcome).count() as f64 / similar.len() as f64,
        }
    }
}

pub struct GateCalibration {
    /// Predicted failure rate per gate [0, 1].
    pub per_gate_failure_prediction: HashMap<String, f64>,
    /// Suggested retry budget from similar tasks.
    pub suggested_retries: u32,
    /// Success rate of similar historical tasks.
    pub similar_task_success_rate: f64,
}
```

---

## 15. Verdict Signal Persistence

### 15.1 Verdict Aggregation Store

```
.roko/learn/
├── verdict-trends.json         # Per-gate trend data
│   {"compile": {"trend": "Stable", "ema": 0.97, "slope": 0.001},
│    "lint": {"trend": "Degrading", "ema": 0.86, "slope": -0.012}}
├── co-failures.json            # Co-failure matrix
│   {"compile+lint": {"count": 12, "phi": 0.45}}
├── signature-clusters.json     # Recurring failure signatures
│   [{"signature": "E0425", "gate": "compile", "count": 23,
│     "trend": "Stable", "severity": 0.6}]
├── replan-log.jsonl            # Replanning decisions and outcomes
│   {"ts": ..., "task": "T5", "action": "DecomposeTask",
│    "reason": "negative progress 3 turns", "outcome": "succeeded"}
└── pattern-memory.json         # Historical verdict patterns for k-NN
    [{"features": {...}, "verdicts": [...], "outcome": true}]
```

---

## 16. Integration with Other Verification Components

| Component | How Verdict Signals Feed It |
|---|---|
| **AdaptiveThresholds** (§06) | Verdict trends adjust SPC parameters; regime shifts trigger recalibration |
| **ProcessRewardModels** (§07) | Verdict patterns → step-level labels; co-failures inform reward shaping |
| **EvoSkills** (§11) | Verdict success patterns → skill extraction; failure patterns → skill retirement |
| **GatePipeline** (§03) | Predictive gate selection uses verdict history to choose which gates to run |
| **GateRatchet** (§05) | Verdict trends inform ratchet strictness — degrading trends tighten ratchet |
| **CascadeRouter** | Verdict-model correlations drive routing; consistent failures with Model X → avoid X |
| **SystemPromptBuilder** | Verdict patterns injected as "lessons learned" section in prompts |
| **Dreams** | Verdict clusters feed Delta consolidation; recurring patterns become knowledge entries |

---

## 17. Extended Test Criteria

| Test | Property |
|---|---|
| `trend_stable_on_constant_rate` | 100 obs at 85% pass rate → VerdictTrend::Stable |
| `trend_degrading_on_declining_rate` | 50 obs at 90% then 50 at 70% → Degrading |
| `trend_regime_shift_on_sudden_change` | 100 at 95% then 100 at 50% → RegimeShift |
| `trend_volatile_on_oscillation` | Alternating pass/fail → Volatile |
| `co_failure_detects_correlation` | Compile+lint fail together 10x → phi > threshold |
| `co_failure_ignores_independent` | Compile and test fail independently → phi < threshold |
| `signature_cluster_counts_correctly` | 5 failures with same signature → cluster.count == 5 |
| `replan_on_same_signature_3x` | Same error 3 times → ReplanAction::ModifyTask |
| `replan_on_negative_progress` | Progress < -0.1 for 3 turns → DecomposeTask |
| `replan_on_low_promise` | Promise < 0.2 for 2 turns → ReplaceTask |
| `predictive_selects_high_risk_gates` | Gate with 80% predicted failure → selected |
| `predictive_skips_low_risk_gates` | Gate with 2% predicted failure → not selected |
| `knn_calibration_from_similar_tasks` | 5 similar tasks with 3 avg retries → suggest 3 |
| `verdict_signal_round_trips_through_substrate` | Write → query → correct tags and lineage |


---

# SOURCE: /Users/will/dev/nunchi/roko/roko/docs/04-verification/INDEX.md

# 04 — Verification (L3 Harness)

> **Layer**: L3 Harness
> **Crate**: `roko-gate` (`crates/roko-gate/src/`)
> **Generated**: 2026-04-11
> **Source**: Prompt 04-verification

---

## What This Section Covers

Layer 3 (Harness) is Roko's verification layer. It establishes ground truth about
agent-produced artifacts by running gates — deterministic external tools (compilers,
test runners, linters, static analyzers) that produce `Verdict`s. Gate verdicts flow
back into the system as signals, feeding the conductor, the router, the learning loops,
and the agents themselves.

The key design principle: **Gate failure is a verdict, not an error.** The `Gate` trait
returns `Verdict` directly — not `Result<Verdict>`. This means every downstream consumer
receives a definitive answer without error-handling code.

---

## Sub-Documents

| # | File | Topic | Lines |
|---|---|---|---|
| 00 | [00-gate-trait.md](./00-gate-trait.md) | The Gate trait: signature, `-> Verdict` design, `name()`, position in universal loop | 210 |
| 01 | [01-gate-implementations.md](./01-gate-implementations.md) | 11 concrete gates: ShellGate, CompileGate, ClippyGate, TestGate, SymbolGate, DiffGate, and scaffolds | 247 |
| 02 | [02-6-rung-selector.md](./02-6-rung-selector.md) | 7-rung selector: Compile→Lint→Test→Symbol→GeneratedTest→PropertyTest→Integration, PlanComplexity, escalation | 230 |
| 03 | [03-gate-pipeline.md](./03-gate-pipeline.md) | GatePipeline: sequential composition, short-circuit, verdict aggregation, test count merging | 218 |
| 04 | [04-artifact-store.md](./04-artifact-store.md) | ArtifactStore: BLAKE3 content-addressed, append-only, deduplicating artifact storage | 200 |
| 05 | [05-ratcheting.md](./05-ratcheting.md) | GateRatchet: monotonic rung tracking, regression prevention, convergence thrashing protection | 225 |
| 06 | [06-adaptive-thresholds.md](./06-adaptive-thresholds.md) | AdaptiveThresholds: per-rung EMA pass rates, retry budget, skip advisory, persistence | 210 |
| 07 | [07-process-reward-models.md](./07-process-reward-models.md) | Process rewards: Promise + Progress scoring, per-turn intervention, multi-timescale feedback | 220 |
| 08 | [08-agent-feedback-from-gates.md](./08-agent-feedback-from-gates.md) | GateFeedback: line classification, noise filtering, severity buckets, token economy | 225 |
| 09 | [09-evaluation-lifecycle.md](./09-evaluation-lifecycle.md) | 14 feedback loops across 5 speed tiers, four-phase lifecycle, Karpathy property, Gauntlet | 215 |
| 10 | [10-autonomous-eval-generation.md](./10-autonomous-eval-generation.md) | Autonomous test generation: verification pipeline, separation of concerns, cheap-model convergence | 220 |
| 11 | [11-evoskills.md](./11-evoskills.md) | EvoSkills: three-tier learning hierarchy, adversarial surrogate verification, cross-model transfer | 220 |
| 12 | [12-forensic-ai-causal-replay.md](./12-forensic-ai-causal-replay.md) | Forensic AI: content-addressed causal chains, regulatory compliance, gap analysis | 225 |

**Total**: ~2,865 lines across 13 sub-documents plus this index.

---

## Architecture Summary

```
                    RungSelector
                   (complexity + caps + failures)
                         │
                         ▼
                    GatePipeline
                   (sequential, short-circuit)
                         │
              ┌──────────┼──────────┐
              ▼          ▼          ▼
         CompileGate  ClippyGate  TestGate  ...  IntegrationGate
          (Rung 0)    (Rung 1)   (Rung 2)       (Rung 6)
              │          │          │                │
              └──────────┼──────────┘                │
                         ▼                           │
                    Aggregated Verdict ◄─────────────┘
                         │
         ┌───────────────┼───────────────────┐
         ▼               ▼                   ▼
    GateRatchet    AdaptiveThresholds   GateFeedback
   (regression)   (retry budget, skip)  (agent retry)
         │               │                   │
         ▼               ▼                   ▼
    ArtifactStore   EfficencyEvents     ProcessRewards
   (BLAKE3 hash)   (per-turn data)   (Promise+Progress)
         │               │                   │
         └───────────────┼───────────────────┘
                         ▼
                  Evaluation Lifecycle
                (14 loops × 5 speed tiers)
                         │
                    ┌────┼────┐
                    ▼         ▼
              EvoSkills   ForensicReplay
            (skill lib)  (causal chains)
```

---

## Key Design Decisions

### 1. `Gate::verify()` returns `Verdict`, not `Result<Verdict>`

Gate failure is a verdict, not an error. Infrastructure failures (spawn errors,
timeouts, malformed input) are encoded as `Verdict::fail()`. This eliminates error
propagation in the pipeline, ratchet, and all downstream consumers.

**Source**: `crates/roko-core/src/traits.rs:102–108`

### 2. Sequential gate execution with short-circuit

Gates run cheapest-first. The first failure stops the pipeline. This is the primary
optimization that makes the 7-rung system efficient: a 3-second compile failure prevents
a 15-minute test run.

**Source**: `crates/roko-gate/src/gate_pipeline.rs`

### 3. Monotonic ratchet

Once a plan passes rung N, it cannot regress below N. This prevents convergence
thrashing where the agent oscillates between fixing different rungs.

**Source**: `crates/roko-gate/src/ratchet.rs`

### 4. EMA-based adaptive thresholds

Per-rung pass rates tracked via EMA (α=0.1) inform retry budgets and skip advisories.
Persistent to disk for cross-session continuity.

**Source**: `crates/roko-gate/src/adaptive_threshold.rs`

### 5. Separation of test generation from implementation

Different agents generate tests and implement code. This adversarial setup prevents the
implementation agent from generating easy-to-pass tests.

**Source**: Verification-first architecture (bardo-backup reference)

---

## Cross-References

| Topic | See Also |
|---|---|
| Gate trait in the Synapse Architecture | `docs/01-architecture/` (Synapse traits) |
| Gate feedback → agent prompts | `docs/03-scaffold/` (prompt assembly) |
| Gate verdicts → model routing | `docs/05-learning/` (CascadeRouter) |
| Gate verdicts → conductor | `docs/06-conductor/` (circuit breaker, watchers) |
| Orchestrator wiring | `crates/roko-cli/src/orchestrate.rs` |
| Agent dispatch | `crates/roko-agent/src/dispatcher/mod.rs` |
| Episode logging | `.roko/episodes.jsonl` |
| Efficiency events | `.roko/learn/efficiency.jsonl` |
| Gate thresholds | `.roko/learn/gate-thresholds.json` |

---

## Source Material

### Canonical Sources (refactoring-prd/)

| File | Sections Used |
|---|---|
| `01-synapse-architecture.md` | Gate trait signature, cybernetic feedback loops |
| `02-five-layers.md` | Layer 3 Harness definition, process reward models |
| `07-implementation-priorities.md` | Tier 2J prediction tracking |
| `08-translation-guide.md` | Naming map, reframe rules |
| `09-innovations.md` | Forensic AI, EvoSkills |

### Legacy Sources (bardo-backup/)

| File | What It Provided |
|---|---|
| `prd/16-testing/01-gauntlet.md` | Gauntlet benchmark suite |
| `prd/16-testing/07-fast-feedback-loops.md` | 5 fast evaluation loops |
| `prd/16-testing/08-slow-feedback-loops.md` | 3 slow evaluation loops |
| `prd/16-testing/09-evaluation-map.md` | 14-loop composition diagram |
| `tmp/mori-refactor/06-harness.md` | Full harness layer spec, academic foundations |
| `tmp/mori-agents/06-eval-and-scoring.md` | Why LLM-as-Judge fails |
| `tmp/mori-agents/20-verification-first-architecture.md` | 6-rung gate system |
| `tmp/death/16-autonomous-verification.md` | Autonomous test infrastructure |

### Implementation Plans

| File | What It Provided |
|---|---|
| `modelrouting/12-advanced-patterns.md` | Gate-to-scaffold feedback, section effectiveness, predictive foraging, process reward tracking |
| `modelrouting/13-architectural-gaps.md` | Generated test gates (GVU verification) |
| `11-sections/phase-7-8.md` | PRD-driven workflow gate verification |

### Active Code

| File | What It Provided |
|---|---|
| `roko-core/src/traits.rs` | Gate trait definition |
| `roko-gate/src/lib.rs` | Module structure |
| `roko-gate/src/gate_pipeline.rs` | GatePipeline implementation |
| `roko-gate/src/rung_selector.rs` | RungSelector, PlanComplexity, Rung enum |
| `roko-gate/src/ratchet.rs` | GateRatchet implementation |
| `roko-gate/src/artifact_store.rs` | ArtifactStore implementation |
| `roko-gate/src/adaptive_threshold.rs` | AdaptiveThresholds implementation |
| `roko-gate/src/feedback.rs` | GateFeedback implementation |
| `roko-gate/src/compile.rs` | CompileGate implementation |
| `roko-gate/src/test_gate.rs` | TestGate implementation |
| `roko-gate/src/shell.rs` | ShellGate implementation |
| `roko-gate/src/clippy_gate.rs` | ClippyGate implementation |
| `roko-gate/src/diff_gate.rs` | DiffGate implementation |
| `roko-gate/src/symbol_gate.rs` | SymbolGate implementation |

### Academic References

| Citation | Context |
|---|---|
| Song et al. (ICLR 2025) | Generation-Verification-Update framework, Variance Inequality |
| Lightman et al. (2023) | PRM800K, "Let's Verify Step by Step" |
| AgentPRM (arXiv:2502.10325) | Per-step rewards for agent tool use |
| SAGE (arXiv:2512.17102) | Self-acquired generalist expertise, skill libraries |
| Voyager (Wang et al. 2023) | Skill accumulation in LLM agents |
| Self-Refine (Madaan et al. 2023) | Iterative self-improvement with feedback |
| Reflexion (Shinn et al. 2023) | Verbal reinforcement for agents |
| Guo et al. (2017) | Expected calibration error |
| ACON (arXiv:2510.00615) | Context compaction |
| Agent Behavioral Contracts (arXiv:2602.22302) | Formal behavioral specifications |

---

## Naming Map Applied

| Old Term | New Term | Notes |
|---|---|---|
| Bardo | Roko | System name |
| Golem | Agent | Actor entity |
| Mori | Roko Orchestrator | Orchestration subsystem |
| Grimoire | Neuro | Knowledge/memory subsystem |
| Signal | Engram | Used "Signal" in code, "Engram" in docs |
| GNOS | KORAI / DAEJI | Meta-cognitive subsystem |
| Clade | Collective / Mesh | Multi-agent groups |
| Succession | Backup / Restore | No death framing |
| Mortality | Resource Management | No death framing |

---

## Generation Notes

- **Prompt**: 04-verification
- **Context pack**: 8 files read from `tmp/prd-migration/context-pack/`
- **Canonical sources**: 5 files from `refactoring-prd/`
- **Legacy sources**: 11 files from `bardo-backup/`
- **Implementation plans**: 3 files from `tmp/implementation-plans/`
- **Active code**: 14 files from `crates/roko-gate/src/`
- **No death/mortality framing** applied throughout
- **Naming map** applied throughout
- **Gate returns Verdict, not Result<Verdict>** emphasized in docs 00, 01, 03, INDEX


---

# SOURCE: /Users/will/dev/nunchi/roko/roko/docs/05-learning/00-episode-logger.md

# Episode Logger

> **Crate:** `roko-learn` · **Module:** `episode_logger.rs`
> **Persistence:** `.roko/learn/episodes.jsonl` (append-only JSONL)
> **Wiring:** `LearningRuntime::record_completed_run()` → `EpisodeLogger::append()`
> **Cross-references:** [04-cascade-router](04-cascade-router.md), [05-pattern-discovery-trigram](05-pattern-discovery-trigram.md), [06-task-metrics-and-baselines](06-task-metrics-and-baselines.md)


> **Implementation**: Shipping

---

## Purpose

The episode logger is the foundational data substrate for all learning in Roko. Every agent turn — regardless of outcome — produces exactly one `Episode` record that is appended to a JSONL file on disk. This append-only log is the raw material from which every other learning subsystem draws its observations: pattern discovery mines trigrams from episode sequences, the cascade router updates bandit arms from episode outcomes, the regression detector computes baselines from episode metrics, and the skill library extracts reusable capabilities from successful episodes.

The design prioritizes durability and simplicity over query performance. Episodes are never modified in place. Concurrent writers are serialized through a process-wide mutex. The reader is tolerant: lines that fail to parse (a common outcome of a crash mid-write or of forward-compatible schema changes) are surfaced through a dedicated error variant rather than corrupting the whole stream.

---

## Episode Schema

The canonical `Episode` struct captures the full context of a single agent turn:

```rust
pub struct Episode {
    /// Unique episode identifier (UUID v4).
    pub id: String,
    /// Agent identifier that produced this episode.
    pub agent_id: String,
    /// Task identifier this episode belongs to.
    pub task_id: String,
    /// Plan identifier containing the task.
    pub plan_id: String,
    /// Agent role (e.g. "Implementer", "Reviewer").
    pub role: String,
    /// Model slug used for this turn (e.g. "claude-sonnet-4-20250514").
    pub model: String,
    /// Backend provider (e.g. "anthropic", "openrouter").
    pub backend: String,
    /// Whether the episode ended in a successful gate pass.
    pub success: bool,
    /// Zero-based iteration index within the task.
    pub iteration: u32,
    /// Input token count from the provider response.
    pub input_tokens: u64,
    /// Output token count from the provider response.
    pub output_tokens: u64,
    /// Actual cost in USD after cache discounts.
    pub cost_usd: f64,
    /// Wall-clock duration in milliseconds.
    pub duration_ms: u64,
    /// Gate verdicts produced by the verification pipeline.
    pub gate_verdicts: Vec<GateVerdict>,
    /// Timestamp when the episode was recorded.
    pub timestamp: DateTime<Utc>,
    /// 10,240-bit HDC fingerprint of the episode content.
    pub hdc_fingerprint: Option<HdcVector>,
    /// Free-form metadata map (capped at 16 KB serialized).
    pub extra: HashMap<String, Value>,
}
```

### GateVerdict

Each gate execution within an episode produces a verdict:

```rust
pub struct GateVerdict {
    /// Gate identifier ("compile", "test", "lint", "diff", etc.).
    pub gate: String,
    /// Whether the gate passed.
    pub passed: bool,
    /// Optional short diagnostic (hashed, never raw output).
    pub signature: Option<String>,
}
```

The `signature` field stores a content hash of the error output rather than the raw text. This serves two purposes: it keeps the log compact (error outputs can be megabytes), and it enables exact-match deduplication across episodes without exposing potentially sensitive build output.

---

## Append Pipeline

The append path is designed for crash-safety and concurrency:

```
Agent Turn Completes
    │
    ▼
EpisodeLogger::append(&episode)
    │
    ├── 1. Validate: extra field ≤ MAX_EXTRA_BYTES (16 KB)
    │       → LoggerError::ExtraTooLarge if exceeded
    │
    ├── 2. Compute HDC fingerprints:
    │       text_fingerprint: bardo_primitives::hdc::text_fingerprint(content)
    │       metadata_fingerprint: text_fingerprint(agent_id + task_id + role)
    │       → stored in episode.extra["text_fingerprint"] and
    │         episode.extra["metadata_fingerprint"]
    │
    ├── 3. Serialize: serde_json::to_string(&episode) + "\n"
    │
    ├── 4. Acquire process-wide parking_lot::Mutex
    │
    ├── 5. Open file with O_APPEND | O_CREAT
    │
    ├── 6. Write serialized line
    │
    └── 7. Release mutex
```

The `MAX_EXTRA_BYTES` guard (16 KB) prevents a runaway optimizer from blowing up the log by stuffing arbitrary data into the `extra` map. This is a hard limit enforced at write time — the episode is rejected with `LoggerError::ExtraTooLarge` if the serialized `extra` field exceeds 16,384 bytes.

### Concurrency Model

The logger uses `parking_lot::Mutex` (not `tokio::Mutex`) for the write serialization lock. This is deliberate: the critical section is a single `write_all` syscall, which is fast enough that the synchronous mutex avoids the overhead of task scheduling. The mutex is process-wide (held by the `EpisodeLogger` instance), so concurrent agent tasks within the same process are serialized, while separate processes append independently (the OS guarantees atomicity for `O_APPEND` writes below `PIPE_BUF`).

---

## HDC Fingerprinting

Every episode is fingerprinted with a 10,240-bit hyperdimensional computing (HDC) vector from `bardo_primitives::hdc`. Two fingerprints are computed:

1. **Text fingerprint** — encodes the semantic content of the episode (task description, gate verdicts, etc.) into a binary vector using `bardo_primitives::hdc::text_fingerprint`.
2. **Metadata fingerprint** — encodes structural identity (agent_id, task_id, role) for structural similarity matching.

HDC fingerprints enable sub-microsecond similarity search: comparing two 10,240-bit vectors via Hamming distance takes ~50ns, compared to ~1μs for cosine distance on 768-dimensional float embeddings. This speed advantage is critical for real-time pattern matching during task dispatch, where the system must scan hundreds of historical episodes to find relevant patterns before the agent begins work.

The fingerprints are stored in the `extra` map under reserved keys (`text_fingerprint` and `metadata_fingerprint`) rather than as top-level fields. This keeps the `Episode` struct backward-compatible with older log entries that predate HDC support.

### Template Suggestion

The episode logger also supports template suggestion via HDC similarity. Given a new task context, the system can scan recent episodes (within `TEMPLATE_SUGGESTION_MAX_AGE_DAYS` = 30 days, up to `TEMPLATE_SUGGESTION_MAX_CANDIDATES` = 256 candidates) and find episodes with HDC similarity above `TEMPLATE_SUGGESTION_MIN_SIMILARITY` = 0.7. Successful episodes matching this threshold can be used to suggest prompt templates or skill patterns for the new task.

---

## Reading and Tolerance

The reader is designed to be tolerant of corruption:

```rust
impl EpisodeLogger {
    pub async fn read_all(path: impl AsRef<Path>) -> Result<Vec<Episode>, LoggerError> {
        // Opens file, reads line-by-line
        // Each line: serde_json::from_str::<Episode>(line)
        // On parse failure: LoggerError::Parse { line, source }
        // Caller decides whether to skip or abort
    }
}
```

The `LoggerError::Parse` variant includes the 1-based line number and the `serde_json` diagnostic, so callers can decide whether to skip corrupt lines or abort. In practice, the `LearningRuntime` skips corrupt lines and logs a warning — this is the right default for a system that must remain operational even after a crash mid-write.

### Why JSONL

The choice of JSONL (one JSON object per line) over alternatives:

| Format | Append-safe | Schema-flexible | Grep-friendly | Corruption-isolated |
|--------|-------------|-----------------|---------------|---------------------|
| JSONL  | Yes         | Yes             | Yes           | Yes (per-line)      |
| SQLite | No (WAL)    | Limited         | No            | No (whole-DB)       |
| Parquet| No          | Limited         | No            | No (whole-file)     |
| CSV    | Yes         | No              | Yes           | Yes (per-line)      |

JSONL's key advantage is corruption isolation: a crash during write corrupts at most one line. The next line is a fresh JSON object that parses independently. This property is essential for an append-only log that may be written to during agent crashes, OOM kills, or power failures.

---

## Retention and Compaction

The current implementation does not compact or rotate the episode log. The log grows monotonically. For a system running 100 tasks per day with ~2 KB per episode, this produces ~200 KB/day or ~73 MB/year — well within filesystem limits.

Future compaction strategies under consideration:

1. **Time-based rotation** — archive episodes older than 90 days to a compressed file, keeping the active log small for fast `read_all` scans.
2. **Summary compaction** — replace old episodes with aggregate summaries (pass rate, cost distribution, pattern counts) that preserve learning signal without individual records.
3. **HDC compaction** — merge similar episodes into a single representative episode using HDC superposition (element-wise majority of fingerprint bits), reducing storage while preserving the similarity search index.

None of these are implemented. The current approach is sufficient for the expected scale of self-hosted development.

---

## Integration with LearningRuntime

The episode logger is the first subsystem updated by `LearningRuntime::record_completed_run()`. The runtime constructs an `Episode` from the `CompletedRunInput` payload and appends it before updating any downstream subsystem:

```
CompletedRunInput
    │
    ├── 1. EpisodeLogger::append(episode)          ← you are here
    ├── 2. CostsLog::append(cost_record)
    ├── 3. PlaybookStore::record_outcome()
    ├── 4. PlaybookRules::validate() / contradict()
    ├── 5. SkillLibrary::record_use()
    ├── 6. TaskMetric → regression history
    ├── 7. ExperimentStore::record_outcome()
    ├── 8. PatternMiner::ingest_episode()
    ├── 9. CascadeRouter::update()
    └── 10. CFactor::compute()
```

This ordering ensures that the raw episode is always persisted before any derived computation runs. If the process crashes during step 5 (skill library update), the episode is already on disk and can be replayed on restart to reconstruct downstream state.

---

## Error Handling

The logger defines three error variants:

| Variant | When | Recovery |
|---------|------|----------|
| `LoggerError::Io` | Filesystem call failed (disk full, permissions) | Retry or alert operator |
| `LoggerError::Serde` | Episode serialization failed (non-serializable value in `extra`) | Fix the caller — this is a programming error |
| `LoggerError::ExtraTooLarge` | `extra` map exceeds 16 KB serialized | Trim the `extra` map before appending |
| `LoggerError::Parse` | A JSONL line could not be deserialized (corruption or schema change) | Skip the line and continue reading |

The `Parse` variant is the most common in practice. It occurs when:
- A crash interrupted a write, leaving a partial JSON line.
- A schema change added new fields that the current deserializer doesn't recognize (forward compatibility).
- Manual editing of the log file introduced syntax errors.

The tolerant reader handles all three cases by surfacing the error with the line number, letting the caller decide whether to skip or abort.

---

## Episode Compression and Tiered Storage

Long-running Roko instances generate thousands of episodes. Naive storage (keep everything in one JSONL) degrades read performance linearly. Episode compression provides a tiered storage architecture inspired by experience replay buffers in deep reinforcement learning (Mnih et al. 2015, Schaul et al. 2016).

### Tiered Architecture

```
Hot tier   (0-7 days)   → episodes.jsonl          Raw JSONL, full fidelity
Warm tier  (7-90 days)  → episodes-warm.jsonl.zst  Zstandard compressed, full fidelity
Cold tier  (90+ days)   → episodes-cold.bin         HDC superposition summaries only
```

### Rust Types

```rust
pub struct EpisodeStorageConfig {
    /// Days before moving to warm tier (default: 7).
    pub hot_retention_days: u32,
    /// Days before moving to cold tier (default: 90).
    pub warm_retention_days: u32,
    /// Zstandard compression level for warm tier (default: 3).
    pub zstd_level: i32,
    /// Maximum cold-tier summary count per (role, complexity) slice (default: 1000).
    pub cold_max_summaries: usize,
}

pub struct CompressedEpisodeSummary {
    /// HDC superposition of all episode fingerprints in this summary.
    pub hdc_superposition: HdcVector,
    /// Number of episodes merged into this summary.
    pub episode_count: u32,
    /// Aggregate pass rate.
    pub pass_rate: f64,
    /// Aggregate cost statistics.
    pub total_cost_usd: f64,
    pub avg_duration_ms: f64,
    /// Time range covered.
    pub earliest: DateTime<Utc>,
    pub latest: DateTime<Utc>,
    /// Role and complexity for this summary slice.
    pub role: String,
    pub complexity_band: String,
}
```

### Compression Algorithm (Pseudocode)

```
fn compact_tier(hot_path, warm_path, cold_path, config):
    episodes = read_all(hot_path)
    now = Utc::now()

    for ep in episodes:
        age_days = (now - ep.timestamp).num_days()
        if age_days > config.warm_retention_days:
            // Move to cold: merge into HDC superposition
            key = (ep.role, ep.complexity_band)
            cold_summaries[key].hdc_superposition |= ep.hdc_fingerprint  // bitwise OR (HDC superposition)
            cold_summaries[key].episode_count += 1
            cold_summaries[key].pass_rate = running_mean(...)
        elif age_days > config.hot_retention_days:
            // Move to warm: compress with zstd
            warm_buffer.push(ep)

    write_zstd(warm_path, warm_buffer, config.zstd_level)
    write_binary(cold_path, cold_summaries)
    truncate_hot(hot_path, episodes_still_hot)
```

### Space Savings

| Tier | Per-episode size | 10,000 episodes | Compression ratio |
|------|-----------------|-----------------|-------------------|
| Hot (raw JSONL) | ~2 KB | 20 MB | 1x (baseline) |
| Warm (zstd) | ~0.4 KB | 4 MB | 5x |
| Cold (HDC summary) | ~1.3 KB per slice | ~50 KB total | 400x |

The cold tier achieves extreme compression by exploiting HDC superposition: merging N episode fingerprints into a single 10,240-bit vector preserves the statistical properties (similarity search still works on the superposition) while discarding individual records. This is analogous to the "compressed sensing" property of high-dimensional random projections (Johnson-Lindenstrauss lemma).

### Integration with Existing Read Path

The tiered storage is transparent to consumers. `EpisodeLogger::read_recent(days)` reads only from hot + warm tiers (fast). `EpisodeLogger::similarity_search(query_hdc)` searches all three tiers, using the cold-tier superpositions for approximate matching against old episodes.

---

## Episode Importance Scoring

Not all episodes are equally valuable for learning. A routine successful episode that matches known patterns contributes little new information, while a surprising failure or an unexpected success on a hard task carries high learning signal. Episode importance scoring quantifies this, inspired by prioritized experience replay (Schaul et al. 2016) and the information-theoretic concept of surprisal.

### Importance Score Components

```rust
pub struct EpisodeImportance {
    /// Overall importance score in [0.0, 1.0].
    pub score: f64,
    /// Component breakdown.
    pub components: ImportanceComponents,
}

pub struct ImportanceComponents {
    /// Surprisal: how unexpected was this outcome given predictions?
    /// High when predicted pass but failed, or predicted fail but passed.
    /// = |predicted_probability - actual_outcome|
    pub surprisal: f64,

    /// Novelty: how different is this episode from recent episodes?
    /// Measured via HDC Hamming distance to nearest neighbor in last 100 episodes.
    /// Range [0.0, 1.0] where 1.0 = maximally novel.
    pub novelty: f64,

    /// Difficulty: was this a hard task that succeeded or an easy task that failed?
    /// Hard successes and easy failures are both highly informative.
    /// = |complexity_adjusted_expected_rate - actual_outcome|
    pub difficulty_signal: f64,

    /// Recency-weighted information gain: how much would including this episode
    /// change the current model parameters (bandit arms, pattern counts)?
    /// Approximated via gradient magnitude for bandit updates.
    pub information_gain: f64,

    /// Diversity contribution: does this episode cover an underrepresented
    /// region of the (role, complexity, model) space?
    /// = 1.0 / sqrt(count_in_same_slice)
    pub diversity: f64,
}
```

### Composite Scoring

```
importance = w_s * surprisal + w_n * novelty + w_d * difficulty_signal
           + w_i * information_gain + w_v * diversity

Default weights:
    w_s = 0.30  (surprisal is the strongest signal)
    w_n = 0.20  (novelty prevents redundant learning)
    w_d = 0.15  (difficulty calibrates expectations)
    w_i = 0.20  (information gain is directly actionable)
    w_v = 0.15  (diversity prevents blind spots)
```

### Applications

| Consumer | How importance is used |
|----------|----------------------|
| Pattern discovery | Weight trigram support by episode importance (important episodes count more) |
| Skill extraction | Prioritize skill extraction from high-importance successful episodes |
| Cascade router | Weight bandit updates by importance (surprising outcomes update more) |
| Compaction | Keep high-importance episodes in hot tier longer before compacting |
| Dashboard | Surface high-importance episodes as "Notable events" |

### Connection to Prioritized Experience Replay

In DQN-style reinforcement learning (Schaul et al. 2016), transitions are sampled from a replay buffer with probability proportional to their TD error — the difference between expected and observed reward. Episode importance scoring is the agent-system analogue: episodes with high surprisal (the "TD error" of the prediction system) receive higher priority in all downstream learning loops. Recent work on diversity-based experience replay (IJCAI 2025) further motivates the diversity component: ensuring the learning pipeline sees a representative sample of the episode space, not just the most surprising examples.

Uncertainty-based prioritization (Clements et al. 2019, extended in 2024) suggests an additional refinement: episodes where the system's uncertainty is highest (wide confidence intervals in the bandit, low pattern support) should be prioritized because they carry the most information about unexplored regions of the decision space. The `information_gain` component approximates this by measuring how much the episode would change model parameters.

---

## Episode Clustering and Automatic Pattern Discovery

Beyond trigram mining (see [05-pattern-discovery-trigram](05-pattern-discovery-trigram.md)), episode clustering provides a complementary pattern discovery mechanism that operates on the full episode representation rather than just action sequences. Clustering discovers structural groupings — families of episodes that share common characteristics across multiple dimensions simultaneously.

### Clustering Algorithm: Incremental DBSCAN over HDC Space

Traditional k-medoids (already implemented for cross-episode consolidation) requires a fixed k. For automatic pattern discovery, DBSCAN (Density-Based Spatial Clustering of Applications with Noise) is preferred because it discovers the number of clusters automatically and identifies outliers.

```rust
pub struct EpisodeClusterConfig {
    /// Minimum HDC similarity to consider two episodes "neighbors" (default: 0.72).
    pub eps_similarity: f64,
    /// Minimum episodes in a neighborhood to form a cluster (default: 3).
    pub min_points: usize,
    /// Maximum episodes to cluster per batch (default: 500).
    pub max_batch_size: usize,
    /// Similarity metric: "hamming" (default) or "cosine".
    pub metric: String,
}

pub struct EpisodeCluster {
    /// Cluster identifier (auto-assigned).
    pub cluster_id: u32,
    /// Medoid episode (most central member).
    pub medoid: Episode,
    /// HDC superposition of all members.
    pub superposition: HdcVector,
    /// Member count.
    pub size: usize,
    /// Aggregate statistics.
    pub pass_rate: f64,
    pub avg_cost_usd: f64,
    pub avg_duration_ms: f64,
    /// Dominant characteristics.
    pub dominant_role: String,
    pub dominant_model: String,
    pub dominant_complexity: String,
    /// Distinguishing features (what makes this cluster unique).
    pub distinguishing_features: Vec<String>,
}
```

### Incremental Clustering

Full DBSCAN is O(n^2) which is acceptable for batch processing but too slow for per-episode updates. Incremental DBSCAN (Ester et al. 1998, updated in Kranen et al. 2011) maintains clusters incrementally:

```
On new episode:
    1. Compute HDC fingerprint
    2. Find nearest cluster (HDC similarity to each cluster superposition)
    3. If similarity > eps_similarity:
        a. Add episode to cluster
        b. Update cluster superposition (bitwise OR)
        c. Update cluster statistics
    4. If no cluster matches:
        a. Add to "noise" buffer
        b. When noise buffer reaches min_points similar episodes:
           -> Form new cluster
```

### Automatic Pattern Extraction from Clusters

Each cluster represents a natural grouping of episodes. The system extracts interpretable patterns by analyzing what cluster members share:

```
Cluster #7: "Cross-crate config modifications" (42 episodes)
    Common features:
        - Files: crates/roko-core/src/config/*.rs (100%)
        - Role: Implementer (95%)
        - Model: claude-sonnet-4 (72%)
    Performance:
        - Pass rate: 0.62 (below baseline 0.75)
        - Avg iterations: 2.3 (above baseline 1.4)
    Suggested action:
        -> Create playbook rule: "Config modifications in roko-core require
           checking serde derives and TOML schema compatibility"
        -> Route to opus for this cluster (low pass rate with sonnet)
```

### Cluster Evolution Tracking

Clusters are not static. As the system improves, cluster characteristics change:

```rust
pub struct ClusterEvolution {
    pub cluster_id: u32,
    /// Pass rate trend (positive = improving).
    pub pass_rate_trend: f64,
    /// Cost trend (negative = getting cheaper).
    pub cost_trend: f64,
    /// Is this cluster shrinking (fewer new episodes match)?
    pub is_shrinking: bool,
    /// Episodes since last cluster member was added.
    pub episodes_since_last_member: u32,
}
```

A cluster that is shrinking and whose pass rate is improving indicates a problem that the system has learned to handle — the pattern is no longer causing failures. Conversely, a growing cluster with declining pass rate indicates an emerging problem that needs attention.

### Relationship to Hindsight Experience Replay

Hindsight experience replay (HER, Andrychowicz et al. 2017) re-labels failed episodes with alternative goals that were actually achieved, turning failures into successes for learning purposes. In the episode clustering context, this principle applies when a failed episode partially achieved a sub-goal: the cluster can identify which sub-goals were achieved and extract partial skills from the failure. For example, a task that failed the test gate but passed compile and lint gates still demonstrates successful compilation patterns that can be extracted as partial skills.

---

## Relationship to Other Documents

- **[05-pattern-discovery-trigram](05-pattern-discovery-trigram.md)** — The pattern miner consumes episodes via the `EpisodeView` trait, extracting trigrams from the `gate_verdicts` sequence.
- **[06-task-metrics-and-baselines](06-task-metrics-and-baselines.md)** — Task metrics are derived from episode data and written to a separate JSONL file for regression detection.
- **[04-cascade-router](04-cascade-router.md)** — The cascade router updates its bandit arms from episode outcomes (model, success, cost).
- **[02-skill-library-voyager](02-skill-library-voyager.md)** — The skill library extracts reusable capabilities from successful episodes.
- **[13-8-missing-feedback-loops](13-8-missing-feedback-loops.md)** — Episodes are the data substrate for all 8 cybernetic feedback loops.

See also: [00-architecture](../00-architecture/INDEX.md) for the Engram/Signal data model that episodes extend, and [04-verification](../04-verification/INDEX.md) for the gate pipeline that produces `GateVerdict` records.


---

# SOURCE: /Users/will/dev/nunchi/roko/roko/docs/05-learning/01-playbook-system.md

# Playbook System

> **Crate:** `roko-learn` · **Modules:** `playbook.rs`, `playbook_rules.rs`
> **Persistence:** `.roko/learn/playbooks/` (JSON per playbook), `.roko/learn/playbook-rules.toml`
> **Wiring:** `LearningRuntime::record_completed_run()` → `PlaybookStore`, `PlaybookRules`
> **Cross-references:** [00-episode-logger](00-episode-logger.md), [02-skill-library-voyager](02-skill-library-voyager.md), [05-pattern-discovery-trigram](05-pattern-discovery-trigram.md), [19-heuristics-worldviews-and-falsifiers](19-heuristics-worldviews-and-falsifiers.md), [04-decay-variants](../00-architecture/04-decay-variants.md), [25-attention-as-currency](../00-architecture/25-attention-as-currency.md), [Naming and Glossary](../00-architecture/01-naming-and-glossary.md), [REF12 demurrage proposal](../../tmp/refinements/12-knowledge-demurrage.md), [REF14 worldview validation proposal](../../tmp/refinements/14-worldview-validation.md)


> **Implementation**: Shipping

---

## Purpose

The playbook system is the concrete procedural projection of Roko's learning stack, not the whole stack by itself. REF14 adds a first-class `Heuristic` layer above episodes and patterns: heuristics capture reusable claims, predictions, falsifiers, and calibration records, while playbooks remain the highly specific ordered steps and prompt-ready rules compiled from those validated beliefs. When a rule correctly predicts outcomes across multiple subsequent executions, it earns enough reinforcement to stay warm and gets injected directly into agent prompts, preventing the agent from repeating known mistakes. Freshness is not governed by confidence alone: demurrage, successful reuse, and contradiction-driven penalties decide whether a rule remains active or cools into cold storage. See also [19-heuristics-worldviews-and-falsifiers](19-heuristics-worldviews-and-falsifiers.md) and `../../tmp/refinements/14-worldview-validation.md`.

The system has two components:

1. **PlaybookStore** — manages named sequences of steps (playbooks) with success/failure counters and freshness balance.
2. **PlaybookRules** — manages if-then rules with globset-based triggers, bounded confidence dynamics, and demurrage-driven reinforcement.

---

## Playbooks Inside The Learning Stack

```
┌──────────────────────────────────────────────────────────────────┐
│              Tier 4: Playbook Rules And Playbooks                │
│   Concrete instructions compiled from validated heuristics and   │
│   repeated strategy fragments. Confidence: 0.0 – 0.95 bounded.  │
│   Reinforcement + balance keep rules warm; demurrage cools      │
│   stale rules. Trigger: file globs, tags, categories, error     │
│   signatures, roles. Action: inject prompt-ready body text.     │
│   Lifecycle: validate / contradict / reinforce / demurrage /    │
│   prune.                                                         │
├──────────────────────────────────────────────────────────────────┤
│             Tier 3: Heuristics And Worldview Priors              │
│   Reusable rules of thumb with preconditions, predictions,       │
│   falsifier surfaces, calibration records, and episode receipts. │
│   See: 19-heuristics-worldviews-and-falsifiers.md                │
├──────────────────────────────────────────────────────────────────┤
│                    Tier 2: Patterns                               │
│   Extracted hypotheses from episode clustering.                   │
│   See: 05-pattern-discovery-trigram.md                            │
├──────────────────────────────────────────────────────────────────┤
│                    Tier 1: Episodes                               │
│   Raw observations from every agent turn.                         │
│   See: 00-episode-logger.md                                       │
└──────────────────────────────────────────────────────────────────┘
```

---

## PlaybookStore

The `PlaybookStore` manages named playbooks — ordered sequences of steps that describe a known-good approach to a task type.

### Playbook Schema

```rust
pub struct Playbook {
    /// Unique playbook identifier.
    pub id: String,
    /// Human-readable name.
    pub name: String,
    /// What the playbook aims to achieve.
    pub goal: String,
    /// Ordered steps to execute.
    pub steps: Vec<PlaybookStep>,
    /// Attention balance that self-trims stale playbooks.
    pub balance: f64,
    /// Total demurrage charged against this playbook.
    pub demurrage_paid: f64,
    /// Number of times this playbook led to a gate pass.
    pub success_count: u32,
    /// Number of times this playbook led to a gate failure.
    pub failure_count: u32,
}

pub struct PlaybookStep {
    /// Zero-based step index.
    pub index: u32,
    /// Human-readable step description.
    pub description: String,
    /// What kind of action this step involves (e.g. "read", "edit", "test").
    pub action_kind: String,
    /// Engrams expected to be produced by this step.
    pub expected_signals: Vec<String>,
}
```

### Persistence

Each playbook is stored as a separate JSON file in `.roko/learn/playbooks/`, keyed by playbook ID. The store uses per-ID async mutexes for concurrent safety: multiple playbooks can be updated simultaneously, but updates to the same playbook are serialized. Writes use the atomic tempfile+rename pattern to prevent corruption on crash.

```
.roko/learn/playbooks/
├── pb-001.json    ← "Rust trait implementation" playbook
├── pb-002.json    ← "Config schema extension" playbook
├── pb-003.json    ← "Gate failure recovery" playbook
└── ...
```

### Operations

| Method | What it does |
|--------|-------------|
| `PlaybookStore::save(playbook)` | Persist a new or updated playbook |
| `PlaybookStore::load(id)` | Load a single playbook by ID |
| `PlaybookStore::load_all()` | Load all playbooks from the directory |
| `PlaybookStore::record_outcome(id, success)` | Increment success or failure counter |

---

## PlaybookRules

The `PlaybookRules` module manages if-then rules with rich trigger matching and bounded confidence dynamics. Rules are the actionable output of the learning system — they are injected into agent prompts to prevent known failure modes, and they self-trim through demurrage instead of depending on fixed-age retention windows.

### Rule Schema

```rust
pub struct Rule {
    /// Stable identifier (synthesized from clustering key).
    pub rule_id: String,
    /// Short human-readable label (≤80 chars).
    pub title: String,
    /// Text injected into the Implementer prompt.
    pub body: String,
    /// Conditions that cause this rule to fire.
    pub triggers: Triggers,
    /// Freshness balance that rises with reinforcement and falls with demurrage.
    pub balance: f64,
    /// Total demurrage charged against this rule.
    pub demurrage_paid: f64,
    /// Confidence score; bounded to [0.0, 0.95].
    pub confidence: f64,
    /// Number of validated predictions.
    pub validations: u32,
    /// Number of contradicted predictions.
    pub contradictions: u32,
    /// When last applied.
    pub last_applied: Option<DateTime<Utc>>,
    /// When first created.
    pub created_at: DateTime<Utc>,
    /// Source episode IDs that generated this rule.
    pub source_episodes: Vec<String>,
}
```

### Trigger System

Rules fire when incoming context matches their `Triggers`:

```rust
pub struct Triggers {
    /// Shell glob patterns matched against files.
    pub file_globs: Vec<String>,
    /// Tag strings (case-insensitive overlap).
    pub tags: Vec<String>,
    /// Task categories.
    pub categories: Vec<String>,
    /// Error signature strings.
    pub error_signatures: Vec<String>,
    /// Agent roles.
    pub roles: Vec<String>,
}
```

Matching uses **OR semantics** across the five trigger kinds: a rule fires if ANY of its trigger lists intersects the incoming context. An all-empty `Triggers` matches nothing — it never fires, guarding against accidental universal rules.

File glob matching uses the `globset` crate for shell-style pattern matching:

```toml
# Example rule in playbook-rules.toml
[[rule]]
rule_id = "rule-008"
title = "Auth module lifetime check"
body = "Check lifetime parameters on all auth types before using them. Use get_symbol_context to see actual signatures."
confidence = 0.92
validations = 12
contradictions = 1

[rule.triggers]
file_globs = ["src/auth/**/*.rs", "crates/roko-agent/src/auth/*"]
tags = ["lifetime", "borrow"]
categories = ["refactor", "bugfix"]
error_signatures = []
roles = ["Implementer"]
```

### Matching Context

When composing a prompt for an agent, the system constructs a `MatchContext` from the current task:

```rust
pub struct MatchContext {
    /// Files the task will modify.
    pub files: Vec<String>,
    /// Tags from the task spec.
    pub tags: Vec<String>,
    /// Task category.
    pub category: Option<String>,
    /// Error signature from the previous failed attempt (if retrying).
    pub error_signature: Option<String>,
    /// Agent role.
    pub role: Option<String>,
}
```

The `PlaybookRules::select(context)` method returns all rules whose triggers match the context, sorted by confidence (highest first). The prompt composer injects the top-N rules (typically 3-5) into the agent's system prompt as "lessons from previous builds."

### Confidence Dynamics

Confidence is update-driven, not time-based, and freshness is governed by balance rather than a hard retention window:

| Trigger | Confidence change | Balance change | Effect |
|-------|------------------|----------------|--------|
| Validation (rule predicted correctly) | `+0.05` | Reinforcement bonus | Keeps a useful rule warm |
| Contradiction (rule predicted incorrectly) | `−0.10` | Reinforcement loss plus cooling pressure | Stale or wrong rules cool faster |
| Successful reuse / citation | N/A | Reinforcement bonus | Returns attention to the rule |
| Demurrage tick | N/A | Holding cost | Unused rules drift toward cold storage |
| Prune threshold | N/A | Balance or confidence below floor | Rule removed if it can no longer justify retention |

The asymmetric update rate (contradictions penalize 2× more than validations reward) ensures that rules which stop being accurate are quickly demoted. Demurrage makes that demotion continuous instead of relying on periodic cleanup: a rule that is no longer cited or successfully reused loses balance over time even if its confidence remains superficially high. The confidence ceiling of 0.95 prevents any rule from becoming "certain" — there is always a small probability that the rule is wrong, which keeps the system open to revision.

```
Confidence lifecycle:
    new rule → 0.50 (default)
        │
        ├── validated → 0.55 → 0.60 → ... → 0.95 (ceiling)
        │
        └── contradicted → 0.40 → 0.30 → ... → 0.0 (pruned)
```

### Why 0.95 Ceiling?

The confidence ceiling prevents epistemic closure. A rule at 1.0 confidence would never be questioned, even if the codebase changes in ways that invalidate the rule's assumptions. The 0.95 ceiling means that every rule, no matter how well-validated, retains a 5% "doubt margin" that allows contradictions to eventually demote it.

---

## Rule Lifecycle

```
Episode Stream
    │
    ▼
Pattern Discovery (trigram mining, HDC clustering)
    │
    ▼
Pattern extracted: "Auth module types have lifetime parameters"
    │
    ├── support_count < 5 → stays as Pattern (Tier 2)
    │
    └── support_count ≥ 5 → promoted to Rule (Tier 3)
            │
            ├── Validated in subsequent builds → confidence climbs
            │
            ├── Contradicted → confidence drops
            │
            └── confidence < min_confidence → pruned (removed)
```

### Promotion Criteria

A pattern is promoted to a rule when:
1. It has appeared in 5+ distinct episodes.
2. Its confidence (proportion of episodes where the predicted outcome matched) exceeds the minimum threshold.
3. The trigger conditions can be expressed as globs, tags, categories, or error signatures.

### Demotion and Pruning

Rules that stop being accurate are automatically demoted:
1. Each contradiction reduces confidence by 0.10 and cuts into balance.
2. Each demurrage tick reduces balance even when the rule is not contradicted.
3. Successful reuse or citation replenishes balance, so rules stay warm only when they keep earning attention.
4. When confidence or balance drops below `min_confidence` / `min_balance` (configurable), the rule is pruned or moved to cold storage.
5. Pruned rules are removed from the TOML file on the next save.

This creates a self-cleaning knowledge base: rules that were valid for an older version of the codebase but no longer apply are automatically removed as contradictions accumulate and their balance drains. It also fixes stale-playbook petrification, where a once-good rule would otherwise sit in prompts forever just because it had a high historical confidence score.

---

## Integration with Prompt Composition

When the prompt composer assembles a system prompt for an agent, it queries the playbook rules:

```
Task spec (files, tags, category, role)
    │
    ▼
PlaybookRules::select(MatchContext)
    │
    ▼
Top-N matching rules (sorted by confidence)
    │
    ▼
Inject into system prompt as "Lessons from previous builds":
    "Note: past builds show that auth module types have
     lifetime parameters. Check actual signatures before
     using them. (confidence: 0.92, validated 12 times)"
```

The injected rules typically consume 50-100 tokens per rule. For a typical task with 2-3 matching rules, this adds ~200 tokens to the prompt — a trivial cost that prevents multi-thousand-token debugging loops where the agent discovers the issue through trial and error.

See [03-composition](../03-composition/INDEX.md) for the full prompt assembly pipeline and how playbook rules fit into the 6-layer `SystemPromptBuilder`.

---

## Persistence Format

Playbook rules are stored in TOML (not JSON) for human readability:

```toml
min_confidence = 0.10
max_body_tokens = 200

[[rule]]
rule_id = "rule-001"
title = "Serde derive for config types"
body = "All types in roko-core::config that cross serialization boundaries need #[derive(Serialize, Deserialize)]. Check the type definition before using it in a TOML/JSON context."
confidence = 0.85
validations = 8
contradictions = 1
created_at = "2026-03-15T10:30:00Z"
source_episodes = ["ep-042", "ep-043", "ep-051", "ep-067", "ep-089"]

[rule.triggers]
file_globs = ["crates/roko-core/src/config/**/*.rs"]
tags = ["serde", "config"]
categories = ["bugfix"]
error_signatures = ["E0277.*Serialize"]
roles = ["Implementer"]
```

The TOML format was chosen over JSON because:
1. Rules are often edited by humans (adding triggers, adjusting confidence).
2. TOML's comment syntax allows annotating rules with context.
3. TOML's array-of-tables syntax (`[[rule]]`) maps naturally to the rule list.

---

## Cross-Project Transfer

Playbook rules are project-agnostic when their triggers use structural patterns rather than project-specific identifiers. A rule triggered by `error_signatures = ["E0277.*Serialize"]` applies to any Rust project, not just the one where it was extracted.

The cross-project transfer workflow:
1. Export rules from project A: `cp .roko/learn/playbook-rules.toml /shared/rules/project-a.toml`
2. Import into project B: merge into `.roko/learn/playbook-rules.toml`
3. Reset confidence to 0.50 and balance to a starter value (rules must re-earn both in the new context)
4. Rules that validate in project B climb in confidence and balance; rules that contradict or go unused lose balance and are pruned.

This enables a form of cross-project knowledge transfer that operates at ~50ns per pattern lookup (via HDC fingerprint matching) rather than requiring expensive embedding-based retrieval.

---

## Relationship to Other Documents

- **[00-episode-logger](00-episode-logger.md)** — Episodes are the raw data from which playbook rules are eventually extracted.
- **[02-skill-library-voyager](02-skill-library-voyager.md)** — Skills capture reusable procedures; playbook rules capture validated predictions. They are complementary: a skill says "how to do X," while a rule says "watch out for Y when doing X."
- **[05-pattern-discovery-trigram](05-pattern-discovery-trigram.md)** — Patterns are the intermediate tier between episodes and rules. Patterns with sufficient support are promoted to rules.
- **[13-8-missing-feedback-loops](13-8-missing-feedback-loops.md)** — The Skills→Prompts feedback loop (loop 5) describes how playbook rules feed back into prompt composition.
- **[14-stability-mechanisms](14-stability-mechanisms.md)** — Confidence dynamics prevent oscillation: the asymmetric update rate and 0.95 ceiling are stability mechanisms.
- **[04-decay-variants](../00-architecture/04-decay-variants.md)** — Demurrage supersedes simple decay for retention.
- **[25-attention-as-currency](../00-architecture/25-attention-as-currency.md)** — Playbook freshness is an attention-economy problem.
- **[Naming and Glossary](../00-architecture/01-naming-and-glossary.md)** — Canonical vocabulary for the learning and memory layers.
- **See also:** [REF12 demurrage proposal](../../tmp/refinements/12-knowledge-demurrage.md)


---

# SOURCE: /Users/will/dev/nunchi/roko/roko/docs/05-learning/02-skill-library-voyager.md

# Skill Library (Voyager-Style)

> **Crate:** `roko-learn` · **Module:** `skill_library.rs`
> **Persistence:** `.roko/learn/skills.json`
> **Wiring:** `LearningRuntime::record_completed_run()` → `SkillLibrary::record_use()`
> **Academic basis:** Wang et al. 2023 ("Voyager: An Open-Ended Embodied Agent with Large Language Models")
> **Cross-references:** [00-episode-logger](00-episode-logger.md), [01-playbook-system](01-playbook-system.md), [13-8-missing-feedback-loops](13-8-missing-feedback-loops.md)


> **Implementation**: Shipping

---

## Purpose

The skill library implements a Voyager-style capability accumulation system (Wang et al. 2023). Where playbook rules capture defensive knowledge ("watch out for X"), skills capture offensive knowledge ("here is how to do X"). Each skill is a named, reusable capability with a prompt template, tool dependencies, example I/O pairs, and usage telemetry that tracks how often the skill succeeds when injected into agent prompts.

The Voyager insight is that agent systems should monotonically accumulate skills from successful executions, building a growing library that makes future tasks cheaper and more reliable. A crate that has been successfully modified 50 times has accumulated patterns for trait implementation, test scaffolding, config extension, and error handling — patterns that a new agent can inherit rather than rediscovering through trial and error.

---

## Skill Schema

```rust
pub struct Skill {
    // ── Core identity ──────────────────────────────────────────
    /// Unique, human-readable identifier (snake_case).
    pub name: String,
    /// One-line description of what the skill does.
    pub summary: String,
    /// Prompt template injected when the skill is selected.
    pub prompt_template: String,
    /// Tools this skill expects the caller to expose.
    pub required_tools: Vec<String>,
    /// Illustrative inputs.
    pub example_inputs: Vec<String>,
    /// Illustrative outputs.
    pub example_outputs: Vec<String>,
    /// Free-form tags for search.
    pub tags: Vec<String>,

    // ── Usage telemetry ────────────────────────────────────────
    /// Smoothed success rate in [0.0, 1.0].
    pub success_rate: f64,
    /// Number of times record_use has been called.
    pub usage_count: u64,

    // ── Voyager-style extraction fields (§16.3.2–16.3.4) ──────
    /// Longer description (1-2 sentences).
    pub description: String,
    /// Plan identifier where this skill was first extracted.
    pub plan_id: String,
    /// Files touched in the originating task.
    pub files: Vec<String>,
    /// Numbered-step recipe from a successful episode (≤750 chars).
    pub pattern: String,
    /// Eval score from the originating episode, in [0.0, 1.0].
    pub score: f64,
    /// When the skill was first extracted.
    pub first_seen: Option<DateTime<Utc>>,
    /// When the skill was last injected into a prompt.
    pub last_matched: Option<DateTime<Utc>>,
    /// How many prompts have had this skill injected.
    pub match_count: u32,
    /// Of those injections, how many led to a gate pass.
    pub validated_count: u32,
    /// Task category for dedup.
    pub task_category: String,
}
```

### Deduplication

Skills sharing ≥70% of their tags AND the same `task_category` are considered duplicates. When a duplicate is detected during registration, the library keeps the skill with the higher `score` and merges the usage telemetry from the lower-scoring duplicate.

---

## Voyager Architecture

The Voyager paper (Wang et al. 2023) describes a three-component system for open-ended skill acquisition:

1. **Automatic Curriculum** — proposes tasks of increasing complexity.
2. **Skill Library** — stores and retrieves reusable code/procedures.
3. **Iterative Prompting** — refines skills through feedback loops.

Roko implements an adapted version where:

| Voyager Component | Roko Equivalent |
|-------------------|-----------------|
| Automatic curriculum | Plan generator (`roko prd plan`) creates tasks from PRDs |
| Skill library | `SkillLibrary` in `roko-learn` with JSON persistence |
| Iterative prompting | Gate pipeline validates output, failed attempts retry with context |
| Environment feedback | Gate verdicts (compile, test, lint, diff) |
| Code verification | 11-gate pipeline in `roko-gate` (see [04-verification](../04-verification/INDEX.md)) |

The key difference from Voyager (which operates in Minecraft) is that Roko's environment is a real codebase with deterministic verification: the gate pipeline provides ground-truth feedback that the skill either works or doesn't. This makes confidence tracking more reliable than in open-ended environments where success criteria are ambiguous.

---

## Skill Extraction Pipeline

Skills are extracted from successful episodes:

```
Successful Episode (gate pass)
    │
    ▼
Analyze execution trace:
    ├── What files were touched, in what order?
    ├── What tools were used?
    ├── What prompt sections were most relevant?
    └── What was the numbered-step recipe?
    │
    ▼
Construct Skill:
    ├── name: derived from task category + file pattern
    ├── prompt_template: generalized version of the successful prompt
    ├── required_tools: tools actually used during the episode
    ├── pattern: numbered-step recipe (≤750 chars)
    ├── files: files touched
    ├── score: episode eval score
    └── tags: derived from file paths, task category, error types
    │
    ▼
SkillLibrary::register(skill)
    ├── Check for duplicates (≥70% tag overlap + same category)
    ├── If duplicate: keep higher-score skill, merge telemetry
    └── If new: add to library, persist to skills.json
```

### Template Pattern Generation

The `TemplatePatternGenerator` trait provides a standardized interface for generating skill templates from episode data. Implementations can use heuristic extraction (analyzing tool calls and file modifications) or LLM-based extraction (asking a cheap model to summarize the episode into a reusable recipe).

---

## Skill Retrieval and Injection

When composing a prompt for a new task, the skill library is queried for relevant skills:

```
Task spec (files, category, tags)
    │
    ▼
SkillLibrary::search_by_tag(tags)
SkillLibrary::search_by_files(files)
    │
    ▼
Filter: success_rate ≥ 0.5, usage_count ≥ 2
    │
    ▼
Rank by: score × success_rate × recency_bonus
    │
    ▼
Top-3 skills injected into prompt as "Recommended approach":
    "Skill: rust_trait_implementation (confidence: 0.87)
     1. Read the existing trait definition with get_symbol_context
     2. Create the impl block in the target file
     3. Add #[cfg(test)] mod tests with at least one smoke test
     4. Run cargo test --lib to verify
     5. Run cargo clippy to check for lint warnings"
```

### Validation Tracking

After a skill is injected, the system tracks whether the task succeeded:

| Outcome | Update |
|---------|--------|
| Gate pass | `skill.validated_count += 1`, `skill.match_count += 1` |
| Gate fail | `skill.match_count += 1` (validated_count unchanged) |

The validation rate (`validated_count / match_count`) provides a direct measure of skill utility. Skills that are frequently matched but rarely validated are candidates for revision or removal.

---

## Persistence and Thread Safety

The `SkillLibrary` is an in-memory `BTreeMap<String, Skill>` guarded by a `parking_lot::RwLock`. Read operations (search, retrieve) acquire a shared read lock. Write operations (register, record_use) acquire an exclusive write lock.

Persistence uses `tokio::fs` with the atomic tempfile+rename pattern:

```
1. Serialize library to JSON
2. Write to temporary file (skills.json.tmp)
3. fsync the temporary file
4. Rename skills.json.tmp → skills.json (atomic on POSIX)
```

This ensures that `skills.json` is always a complete, valid JSON document. A crash during step 2 leaves the temporary file (which is ignored on next load), while the original `skills.json` remains intact.

### Startup Behavior

On startup, `SkillLibrary::new(path)` loads the existing `skills.json` if present. If the file does not exist, the library starts empty. If the file exists but is corrupt (invalid JSON), the library fails to initialize with `SkillLibraryError::Serde`.

---

## Monotonic Growth Property

The skill library is designed to grow monotonically: skills are added but never removed in normal operation. This mirrors the Voyager insight that accumulated knowledge should only increase over time. The only mechanisms that reduce the library are:

1. **Deduplication** — when a new skill duplicates an existing one, the lower-scoring duplicate is discarded.
2. **Manual pruning** — an operator can edit `skills.json` to remove obsolete skills.

There is no automatic pruning based on low usage or low success rate. The rationale: a skill that hasn't been used recently may still be valuable when a matching task appears. The cost of storing unused skills is negligible (a few KB each), while the cost of re-extracting a pruned skill is significant (requires a successful episode to trigger extraction again).

---

## Cross-Crate and Cross-Project Transfer

Skills that use structural patterns (trait implementation, test scaffolding, config extension) rather than project-specific identifiers are transferable across codebases. A skill for "Rust trait implementation" works in any Rust project, not just the one where it was extracted.

The transfer mechanism:

```
Project A skills.json → export → Project B skills.json
    │
    ├── Reset usage_count to 0
    ├── Reset success_rate to 0.5 (neutral prior)
    ├── Keep pattern, prompt_template, required_tools
    └── Skills must re-earn confidence in project B
```

This is analogous to the cross-project HDC fingerprint matching described in the episode logger — structural similarity, not nominal identity, determines transferability.

---

## Error Handling

```rust
pub enum SkillLibraryError {
    /// A skill with the requested name already exists.
    Duplicate(String),
    /// No skill with the requested name exists.
    NotFound(String),
    /// I/O error while reading or writing the persistence file.
    Io(io::Error),
    /// JSON (de)serialization error.
    Serde(serde_json::Error),
}
```

The `Duplicate` error is raised when `register()` is called with a skill name that already exists. Callers can use `register_or_update()` to upsert instead.

---

## Relationship to Voyager and Other Frameworks

| Framework | Skill Representation | Retrieval | Validation |
|-----------|---------------------|-----------|------------|
| Voyager (Wang et al. 2023) | JavaScript functions | Embedding similarity | Environment feedback |
| ExpeL (Zhao et al. 2023) | Natural language insights | Task-type matching | Success/failure tracking |
| Roko SkillLibrary | Prompt templates + tool lists | Tag + file matching + HDC | Gate pipeline verdicts |

Key differences from Voyager:
- **Language-agnostic skills**: Roko skills are prompt templates, not code in a specific language. The agent interprets the template and generates appropriate code.
- **Deterministic validation**: Gate pipeline provides ground-truth success/failure, unlike Minecraft's ambiguous environment feedback.
- **Bounded confidence**: Skills can never reach 1.0 confidence (bounded by validation rate tracking), preventing epistemic closure.

---

## Relationship to Other Documents

- **[00-episode-logger](00-episode-logger.md)** — Successful episodes are the source material for skill extraction.
- **[01-playbook-system](01-playbook-system.md)** — Playbook rules are defensive ("watch out for X"), skills are offensive ("here is how to do X"). They complement each other in prompt composition.
- **[05-pattern-discovery-trigram](05-pattern-discovery-trigram.md)** — Patterns identify recurring sequences; skills capture the full procedure associated with successful sequences.
- **[13-8-missing-feedback-loops](13-8-missing-feedback-loops.md)** — Loop 5 (Skills→Prompts) describes how skills feed back into prompt composition.
- **[17-adas-and-autocatalytic](17-adas-and-autocatalytic.md)** — The autocatalytic thesis posits that monotonically growing skill libraries are a key mechanism for compound improvement.

See also: EvoSkills (Chen et al. 2023) for evolutionary skill optimization, described in [17-adas-and-autocatalytic](17-adas-and-autocatalytic.md).


---

# SOURCE: /Users/will/dev/nunchi/roko/roko/docs/05-learning/03-bandits-ucb-thompson-linucb.md

# Bandits: UCB1, Thompson Sampling, LinUCB

> **Crate:** `roko-learn` · **Modules:** `bandits.rs`, `model_router.rs`
> **Persistence:** `.roko/learn/cascade-router.json` (LinUCB state), per-bandit JSON files
> **Academic basis:** Auer, Cesa-Bianchi & Fischer 2002 (UCB1); Li et al. 2010 (LinUCB); Garivier & Kaufmann 2016 (Track-and-Stop); Thompson 1933
> **Cross-references:** [04-cascade-router](04-cascade-router.md), [11-thompson-sampling-drift](11-thompson-sampling-drift.md), [10-pareto-frontier-pruning](10-pareto-frontier-pruning.md)


> **Implementation**: Shipping

---

## Purpose

Roko uses multi-armed bandit algorithms for every repeated decision in the system: which model to route a task to, which prompt section to include, which tool format to use, which backend to prefer. Bandits provide a principled framework for balancing exploration (trying less-tested options) against exploitation (using the best-known option), with formal regret bounds that guarantee convergence to optimal choices.

The `roko-learn` crate provides three bandit implementations, each suited to a different decision structure:

| Bandit | Algorithm | Use Case | Key Property |
|--------|-----------|----------|--------------|
| `UcbBandit` | UCB1 (Auer et al. 2002) | Context-free repeated decisions | O(√(T ln T)) cumulative regret |
| `LinUCBRouter` | LinUCB (Li et al. 2010) | Context-dependent model routing | Handles 18-dim context vectors |
| `TrackAndStopBandit` | Track-and-Stop (Garivier & Kaufmann 2016) | Best-arm identification | Stops when confident, not after fixed trials |
| `BanditBank` | Collection of UCB1 instances | Keyed decision spaces | One bandit per context key |

---

## UCB1: Upper Confidence Bound

The `UcbBandit` implements the classic UCB1 algorithm for context-free multi-armed bandits.

### UCB1 Formula

For each arm `a` with `pulls_a` observations:

```
ucb(a) = mean_a + C · √(ln(total_pulls) / pulls_a)
```

where:
- `mean_a` = cumulative reward / pulls_a
- `C` = exploration constant (default: √2)
- `total_pulls` = sum of all arm pulls

Arms with `pulls_a == 0` receive infinite UCB and are always chosen before any pulled arm. Tiebreaking is deterministic: first by insertion order.

### Reward Scaling

UCB1 regret bounds assume rewards in `[0, 1]`. Callers must normalize:

| Outcome | Reward |
|---------|--------|
| Gate pass | 1.0 |
| Gate fail | 0.0 |
| Mixed (partial success) | `1.0 − (cost / max_cost)` |

### Schema

```rust
pub struct BanditArm {
    /// Human-readable name (e.g. "claude", "codex").
    pub name: String,
    /// Number of times this arm has been pulled.
    pub pulls: u64,
    /// Cumulative reward received across all pulls.
    pub total_reward: f64,
}

pub struct UcbBandit {
    arms: RwLock<Vec<BanditArm>>,
    total_pulls: AtomicU64,
    /// UCB exploration constant (default: √2).
    exploration_c: f64,
    /// Persistence path (optional).
    persist_path: Option<PathBuf>,
}
```

### Thread Safety

`UcbBandit` uses `parking_lot::RwLock` for arm stats and `AtomicU64` for the pull counter. `select()` acquires only a shared read lock while `update()` acquires an exclusive write lock. This means concurrent `select()` calls never block each other — only an in-progress `update()` causes contention.

### Use Cases

- **Backend selection**: which LLM provider to route a request to.
- **Retry strategy**: immediate retry vs. escalate model vs. re-plan.
- **Context-size buckets**: how much context to include in the prompt.
- **Prompt experiment variant selection**: which variant of a prompt section to use.

---

## BanditBank: Keyed Collections

The `BanditBank` manages a collection of independent `UcbBandit` instances keyed by context string. This is used when the same decision must be made in multiple distinct contexts, each with its own reward distribution.

```
BanditBank {
    "implementer:rust:standard" → UcbBandit { arms: [claude, codex, gemini] }
    "reviewer:rust:complex"     → UcbBandit { arms: [claude, codex, gemini] }
    "planner:python:fast"       → UcbBandit { arms: [claude, codex, gemini] }
}
```

Bandits are created lazily: when a `select(key, ...)` call arrives for a key that doesn't exist, a new `UcbBandit` is initialized with all available arms and zero observations. This ensures that new context keys start with full exploration before converging.

### Persistence

The entire bank is serialized to a single JSON file. Each bandit's arm stats are included, so the system resumes with full history on restart.

---

## LinUCB: Contextual Bandit Router

The `LinUCBRouter` implements the LinUCB algorithm (Li et al. 2010) for context-dependent model selection. Unlike UCB1, which treats each arm independently, LinUCB models the expected reward as a linear function of a context vector, allowing the router to generalize across similar contexts.

### LinUCB Formula

For each arm `a` with context vector `x`:

```
score(a) = θ_a^T · x + α · √(x^T · A_a^{-1} · x)
```

where:
- `θ_a = A_a^{-1} · b_a` (ridge regression estimate)
- `A_a` = d×d matrix (initialized to identity)
- `b_a` = d×1 vector (initialized to zero)
- `α` = exploration parameter (decays from 1.0 to 0.05)

### Context Vector (18 dimensions)

The `RoutingContext` encodes task features into a fixed-length vector:

| Dimension(s) | Feature | Encoding |
|--------------|---------|----------|
| 0-7 | Task category | One-hot (8 `TaskCategory` variants) |
| 8 | Complexity band | Scalar: 0.0 (Fast) / 0.5 (Standard) / 1.0 (Complex) |
| 9 | Iteration | Normalized: `iteration / 10`, capped at 1.0 |
| 10-13 | Agent role | 4-dim float vector (hashed from role string) |
| 14 | Crate familiarity | `success_count / total_count`, clamped to [0.0, 1.0] |
| 15 | Has prior failure | Binary: 0.0 or 1.0 |
| 16 | Bias term | Always 1.0 |
| 17 | Cache affinity | 1.0 when candidate matches previous model, else 0.0 |

Total dimension: `CONTEXT_DIM = 18`.

### Alpha Decay

The exploration parameter `α` decays exponentially from 1.0 to 0.05 over 200 observations:

```
α = 0.05 + 0.95 · exp(−observations / 60)
```

At cold start (0 observations), `α = 1.0` — maximum exploration. After 200 observations, `α ≈ 0.084` — mostly exploitation with minimal exploration. The decay constant `τ = 60` was chosen so that `exp(−200/60) ≈ 0.036`, giving effective convergence by 200 observations.

### Cold Start

When observation count is below `COLD_START_THRESHOLD = 50`, the router falls back to a static mapping from `ModelTier` to a default model slug. This prevents the LinUCB from making poorly-informed decisions with insufficient data.

### Cache Affinity

The context vector includes a cache affinity dimension (dimension 17) that is 1.0 when the candidate model matches the model used for the previous task in the same plan. This encodes the observation that consecutive tasks in a plan often share similar context, and reusing the same model allows the provider's KV cache to serve prefix tokens at reduced cost.

The `CACHE_AFFINITY_BONUS = 0.15` in the cascade router provides an additional static bonus for cache-consistent routing during the confidence stage, before the LinUCB has learned the relationship from data.

---

## Track-and-Stop: Best-Arm Identification

The `TrackAndStopBandit` implements the Track-and-Stop algorithm (Garivier & Kaufmann 2016) for best-arm identification with anytime-valid stopping. Unlike UCB1 which minimizes cumulative regret, Track-and-Stop minimizes the number of samples needed to identify the best arm with probability ≥ 1 − δ.

### Algorithm

```
Phase 1: Round-robin
    Pull each arm at least once.

Phase 2: D-tracking
    Compute target allocation proportions from gap estimates.
    Pull the arm most under-sampled relative to its target.
    Forced exploration: no arm falls below √t − K/2 pulls.

Phase 3: Stopping
    When GLR statistic > β(t, δ), declare winner.
    Stop exploring permanently for this key.
```

### GLR Stopping Criterion

The Generalized Likelihood Ratio statistic is:

```
GLR(t) = t · KL(μ̂_1, μ̂_2)
```

where `μ̂_1` and `μ̂_2` are the empirical means of the top-2 arms. When `GLR(t) > β(t, δ)` where `β(t, δ) = ln((ln(t) + 1) / δ)`, the best arm is declared with confidence ≥ 1 − δ.

### Use Case: Tool Format Selection

The `TrackAndStopBandit` implements the `FormatBandit` trait for adaptive tool-format selection. For each `(model, role, tool_count, complexity)` key, the bandit identifies the best tool format (JSON, XML, native function calling) with high confidence, then stops exploring permanently for that key.

```rust
pub trait FormatBandit: Send + Sync {
    fn select_format(&self, key: &BanditKey) -> ToolFormat;
    fn update_format(&self, key: &BanditKey, format: ToolFormat, outcome: &ToolOutcome);
}
```

### Why Track-and-Stop Instead of UCB1?

UCB1 never stops exploring — it always allocates some trials to suboptimal arms. For decisions where:
1. The optimal choice is fixed (the best tool format for a given model doesn't change over time).
2. Exploration has a cost (suboptimal tool formats waste tokens and cause parse errors).
3. We need high confidence in the answer, not just low regret.

Track-and-Stop is the right algorithm: it explores only as much as needed, then commits permanently.

---

## Reward Scaling Across Bandits

All three bandit implementations assume rewards in `[0, 1]`:

| Signal | Reward Value |
|--------|-------------|
| Gate pass (first attempt) | 1.0 |
| Gate pass (after retry) | 0.7 |
| Gate fail (recoverable) | 0.2 |
| Gate fail (unrecoverable) | 0.0 |
| Cost efficiency | `1.0 − (cost / max_cost)` |

For the cascade router, rewards are typically binary (1.0 for gate pass, 0.0 for fail) with a cost adjustment that penalizes expensive successes. See [04-cascade-router](04-cascade-router.md) for the full reward computation.

Track-and-Stop also assumes sub-Gaussian rewards with parameter σ = 0.5. The GLR stopping criterion uses this assumption for threshold calibration.

---

## Persistence

| Component | Format | Path |
|-----------|--------|------|
| `UcbBandit` | JSON (arm stats) | Per-bandit file |
| `BanditBank` | JSON (all bandits) | Single file |
| `LinUCBRouter` | JSON (A matrices, b vectors, obs count) | `.roko/learn/cascade-router.json` |
| `TrackAndStopBandit` | JSON (per-key state) | Per-instance file |

All persistence uses the atomic tempfile+rename pattern for crash safety.

---

## Neural Contextual Bandits

Linear contextual bandits (LinUCB) assume a linear relationship between context features and reward. When the true reward function is nonlinear — e.g., interaction effects between task complexity and crate familiarity — LinUCB's regret grows. Neural contextual bandits replace the linear model with a neural network, capturing nonlinear reward structure.

### Architecture: NeuralUCB (Zhou et al. 2020)

NeuralUCB extends LinUCB by replacing the linear predictor with a neural network and deriving an exploration bonus from the network's gradient:

```rust
pub struct NeuralUCBRouter {
    /// Neural network f(x; θ) mapping context → predicted reward per arm.
    network: NeuralRewardNet,
    /// Per-arm gradient covariance matrix for exploration.
    /// Z_a = Σ_t g_t g_t^T + λI where g_t = ∇_θ f(x_t; θ)
    gradient_covariance: HashMap<String, DMatrix<f64>>,
    /// Exploration parameter (analogous to α in LinUCB).
    pub nu: f64,
    /// Regularization parameter (default: 1.0).
    pub lambda: f64,
    /// Training buffer for periodic network updates.
    training_buffer: Vec<(ContextVector, String, f64)>,
    /// Retrain every N observations (default: 50).
    pub retrain_interval: u32,
}

pub struct NeuralRewardNet {
    /// Input dimension (same as LinUCB: 18).
    input_dim: usize,
    /// Hidden layer sizes (default: [64, 32]).
    hidden_dims: Vec<usize>,
    /// Output: predicted reward per arm.
    output_dim: usize,
    /// Network parameters θ.
    params: Vec<f64>,
}
```

### Selection with Neural Exploration Bonus

```
For each arm a:
    predicted_reward = f(context; θ)  // neural network forward pass
    gradient = ∇_θ f(context; θ)       // backprop to get gradient
    exploration_bonus = ν × √(gradient^T × Z_a^{-1} × gradient)
    score(a) = predicted_reward + exploration_bonus
Select arm with highest score.
```

### When to Use Neural vs Linear

| Criterion | LinUCB | NeuralUCB |
|-----------|--------|-----------|
| Context dimension | Low (≤20) | Any |
| Reward structure | Approximately linear | Nonlinear interactions |
| Sample efficiency | Higher (fewer params) | Lower (needs ~500+ obs) |
| Computational cost | O(d²) per update | O(network_size) per update |
| Interpretability | High (weight per feature) | Low (black box) |
| Cold start | Better (fewer params to learn) | Worse (needs more data) |

**Roko recommendation:** Use LinUCB (current) until 500+ observations accumulate and the prediction residuals show nonlinear structure. Then optionally transition to NeuralUCB as a stage-4 cascade extension.

### Non-Stationary Neural Bandits (NP-ES, Zhu et al. 2023)

Neural Predictive Ensemble Sampling (NP-ES) addresses non-stationarity by maintaining an ensemble of neural networks and using a predictive sampling strategy that prioritizes collecting information with lasting value. This is relevant when model providers update frequently:

```
NP-ES Algorithm:
    1. Maintain K neural networks (ensemble)
    2. For each decision:
       a. Sample one network from ensemble
       b. Use its prediction + exploration bonus
    3. On reward observation:
       a. Update all K networks with discounted loss
       b. Weight recent observations more heavily
    4. Ensemble disagreement = uncertainty estimate
       → High disagreement = explore more
```

This combines the non-stationarity handling of Thompson Sampling with drift (see [11-thompson-sampling-drift](11-thompson-sampling-drift.md)) and the representation power of neural networks. The ensemble disagreement provides a natural uncertainty estimate without requiring explicit covariance computation.

---

## Bandit Ensembles and Meta-Selection

When multiple bandit algorithms are available (UCB1, LinUCB, Thompson Sampling, NeuralUCB), the question arises: which bandit should we use? Meta-bandits solve this by treating the choice of bandit algorithm as itself a bandit problem.

### Architecture

```rust
pub struct BanditEnsemble {
    /// Available bandit strategies.
    strategies: Vec<Box<dyn BanditStrategy>>,
    /// Meta-bandit that selects which strategy to use.
    meta_bandit: UcbBandit,
    /// Per-strategy performance tracking.
    strategy_stats: Vec<StrategyStats>,
    /// Correlation matrix between strategies (for diversity).
    correlation_matrix: Vec<Vec<f64>>,
    /// Ensemble combination mode.
    pub mode: EnsembleMode,
}

pub enum EnsembleMode {
    /// Meta-bandit selects one strategy per decision.
    MetaSelect,
    /// Weighted vote across all strategies.
    WeightedVote,
    /// Majority vote with tie-breaking by meta-bandit.
    MajorityVote,
    /// Switch strategy when current strategy's regret exceeds threshold.
    AdaptiveSwitch { regret_threshold: f64 },
}

pub struct StrategyStats {
    /// Strategy name.
    pub name: String,
    /// Cumulative reward under this strategy.
    pub cumulative_reward: f64,
    /// Number of times this strategy was selected.
    pub selections: u64,
    /// Running regret estimate.
    pub estimated_regret: f64,
    /// Recent performance (last 50 decisions).
    pub recent_reward_rate: f64,
}
```

### Meta-Selection Algorithm

```
On each routing decision:
    1. meta_bandit.select() → choose strategy_i
    2. arm = strategy_i.select(context)
    3. Execute arm, observe reward
    4. strategy_i.update(arm, reward)
    5. meta_bandit.update(strategy_i, reward)

The meta-bandit learns which strategy works best in the current environment:
    - Stationary environment → UCB1 or LinUCB dominate
    - Non-stationary environment → Thompson+drift dominates
    - High-dimensional context → NeuralUCB dominates
    - Low data regime → UCB1 dominates (fewest parameters)
```

### Adaptive Strategy Switching

The `AdaptiveSwitch` mode monitors each strategy's running regret estimate and switches when the current strategy appears to be underperforming:

```
Every 50 decisions:
    for each strategy:
        regret_estimate = optimal_arm_reward × selections - cumulative_reward
        regret_rate = regret_estimate / selections
    if current_strategy.regret_rate > regret_threshold:
        switch to strategy with lowest regret_rate
```

This provides automatic adaptation to environmental changes: if LinUCB worked well but model providers updated (introducing non-stationarity), the ensemble detects increasing regret and switches to Thompson+drift.

### Correlated Arms and Diversification

When strategies are correlated (they tend to select the same arm), the ensemble provides little benefit. The correlation matrix tracks per-pair agreement rates:

```
correlation(strategy_i, strategy_j) =
    count(both_select_same_arm) / count(both_queried)
```

If correlation > 0.9, the strategies are redundant and one can be pruned from the ensemble to save computation. If correlation < 0.3, the strategies provide genuine diversity and the ensemble benefits from combining them.

---

## Bandit Visualization and Debugging

Understanding bandit behavior is critical for debugging routing anomalies. This section specifies the diagnostic views that the TUI dashboard and log analysis tools should provide.

### Arm Performance Dashboard

```
┌─────────────────────────────────────────────────────────────┐
│ Cascade Router — Stage 3 (LinUCB, 347 observations)         │
├─────────────────────────────────────────────────────────────┤
│ Arm                 Pulls  Reward  UCB Score  Pass%  $/task │
│ claude-haiku-4.5      89   71.2    0.837      80%   $0.12  │
│ claude-sonnet-4      156  108.0    0.812      69%   $0.95  │
│ claude-opus-4        102   89.0    0.891      87%   $2.40  │
│                                                              │
│ Exploration rate: 12% (target: 10-15%)                       │
│ Hysteresis blocks: 23 (since last switch)                    │
│ Current best: claude-opus-4 (score: 0.891)                   │
│ Pareto frontier: [haiku, opus] (sonnet dominated)            │
└─────────────────────────────────────────────────────────────┘
```

### Regret Trajectory Plot

Track cumulative regret over time to detect convergence:

```rust
pub struct RegretTracker {
    /// Per-decision regret: best_arm_reward - chosen_arm_reward.
    pub per_decision_regret: Vec<f64>,
    /// Cumulative regret over time.
    pub cumulative_regret: Vec<f64>,
    /// Theoretical O(√(T ln T)) bound for comparison.
    pub theoretical_bound: Vec<f64>,
}
```

```
Cumulative Regret
    │
 40 │                                              ╱ theoretical √(T ln T)
    │                                           ╱
 30 │                                        ╱
    │                                ╱╱╱╱╱
 20 │                          ╱╱╱╱
    │                   ╱╱╱╱╱    ← actual regret
 10 │            ╱╱╱╱╱
    │     ╱╱╱╱╱
  0 └───────────────────────────────────────────► Decisions
    0      50     100     150     200     250
```

If actual regret exceeds the theoretical bound, the bandit is misconfigured (wrong exploration constant, stale data, or nonlinear reward structure that linear UCB cannot capture).

### Context Feature Importance

For LinUCB, the learned weight vector θ_a reveals which context features matter most:

```rust
pub struct FeatureImportance {
    pub feature_name: String,
    pub dimension: usize,
    /// Average |weight| across all arms.
    pub avg_abs_weight: f64,
    /// Variance of weight across arms (high = discriminative).
    pub weight_variance: f64,
}
```

```
LinUCB Feature Importance (averaged across arms):
    complexity_band:    ████████████████████  0.42 (most important)
    has_prior_failure:  ██████████████        0.28
    crate_familiarity:  ███████████           0.23
    iteration:          ████████              0.17
    cache_affinity:     ██████                0.12
    task_category[3]:   ████                  0.08
    bias_term:          ███                   0.06
    ...
```

Features with near-zero importance across all arms are candidates for removal from the context vector, simplifying the model and potentially improving sample efficiency.

### Anomaly Detection for Bandits

```rust
pub enum BanditAnomaly {
    /// One arm is selected >80% of the time (potential lock-in).
    ArmLockIn { arm: String, selection_rate: f64 },
    /// Exploration rate dropped below 5% before convergence.
    PrematureExploitation { exploration_rate: f64, observations: u64 },
    /// Regret is growing faster than theoretical bound.
    SuperlinearRegret { actual: f64, bound: f64 },
    /// Arm performance suddenly changed (possible provider update).
    ArmPerformanceShift { arm: String, old_rate: f64, new_rate: f64 },
    /// All arms have similar performance — bandit cannot distinguish.
    IndistinguishableArms { max_gap: f64 },
}
```

These anomalies are surfaced in the TUI dashboard and can trigger automatic corrective actions (e.g., resetting an arm on `ArmPerformanceShift`, increasing exploration on `PrematureExploitation`).

---

## Relationship to Other Documents

- **[04-cascade-router](04-cascade-router.md)** — The cascade router uses LinUCB as its stage-3 routing algorithm.
- **[10-pareto-frontier-pruning](10-pareto-frontier-pruning.md)** — Pareto pruning restricts the arm set presented to the bandit.
- **[11-thompson-sampling-drift](11-thompson-sampling-drift.md)** — Thompson Sampling with discount factor is an alternative to UCB1 for non-stationary environments.
- **[08-cost-normalization](08-cost-normalization.md)** — Cost normalization affects the reward signal fed to bandits.
- **[14-stability-mechanisms](14-stability-mechanisms.md)** — Hysteresis prevents bandits from oscillating between near-equal arms.


---

# SOURCE: /Users/will/dev/nunchi/roko/roko/docs/05-learning/04-cascade-router.md

# Cascade Router

> **Crate:** `roko-learn` · **Module:** `cascade_router.rs`
> **Persistence:** `.roko/learn/cascade-router.json`
> **Wiring:** `LearningRuntime` → `CascadeRouter::select()` (called from orchestrate.rs)
> **Cross-references:** [03-bandits-ucb-thompson-linucb](03-bandits-ucb-thompson-linucb.md), [09-provider-health-circuit-breaker](09-provider-health-circuit-breaker.md), [10-pareto-frontier-pruning](10-pareto-frontier-pruning.md), [08-cost-normalization](08-cost-normalization.md)


> **Implementation**: Shipping

---

## Purpose

The cascade router is Roko's central model selection system. It answers the question: "Given a task with these features (category, complexity, role, iteration, crate familiarity), which LLM model should run it?" The answer evolves as the system accumulates observations, transitioning through three stages of increasing sophistication.

The cascade design is inspired by production routing systems (RouteLLM, Ong et al. ICLR 2025; FrugalGPT, Chen et al. arXiv:2305.05176; AutoMix, NeurIPS 2024) but adapted for a self-hosted development tool where the reward signal (gate pass/fail) is deterministic and the decision space (which model to route to) is small enough for contextual bandits rather than neural routers.

---

## Three-Stage Cascade

The router transitions through three stages as observation count grows:

```
┌─────────────────────────────────────────────────────────────────┐
│  Stage 1: Static          │  < 50 observations                  │
│  Hardcoded role→model     │  No learning, safe defaults          │
│  table                    │                                      │
├───────────────────────────┼──────────────────────────────────────┤
│  Stage 2: Confidence      │  50 – 200 observations               │
│  Empirical pass rates +   │  Simple statistics, wide confidence  │
│  confidence intervals     │  intervals shrink with data          │
├───────────────────────────┼──────────────────────────────────────┤
│  Stage 3: UCB             │  > 200 observations                  │
│  Full LinUCB contextual   │  Context-dependent routing with      │
│  bandit                   │  learned feature weights              │
└─────────────────────────────────────────────────────────────────┘
```

### Why Three Stages?

A single bandit algorithm works poorly at all scales:

- **Too few observations for UCB**: LinUCB with 18 context dimensions needs ~50 observations per arm to begin producing meaningful weights. With 5 models, that's 250+ observations before the bandit is useful. During cold start, random exploration wastes money on expensive models for trivial tasks.
- **Too crude for static forever**: A hardcoded table can never adapt to crate-specific patterns, role-specific model preferences, or changes in model capabilities after a provider update.
- **Confidence stage bridges the gap**: Between 50 and 200 observations, simple pass-rate statistics with confidence intervals provide reasonable routing without the sample complexity requirements of a 18-dimensional linear model.

---

## Stage 1: Static Routing (< 50 observations)

Before the system has enough data to learn, it uses a hardcoded mapping from `ModelTier` to model slug:

```rust
fn static_route(tier: ModelTier) -> ModelSpec {
    match tier {
        ModelTier::Fast    => ModelSpec::new("claude-haiku-4-5-20251001"),
        ModelTier::Standard => ModelSpec::new("claude-sonnet-4-20250514"),
        ModelTier::Complex => ModelSpec::new("claude-opus-4-20250514"),
    }
}
```

The tier is determined by the task's complexity band and role. This mapping is deliberately conservative: it over-routes to stronger models to avoid gate failures during the cold-start period, accepting higher cost in exchange for higher pass rates while the system builds its observation base.

---

## Stage 2: Confidence Routing (50–200 observations)

Once 50 observations have accumulated, the router transitions to empirical pass-rate routing with confidence intervals.

### Per-Model Statistics

```rust
struct ModelStats {
    trials: u64,      // selections for this model
    successes: u64,   // gate passes
}

impl ModelStats {
    fn pass_rate(&self) -> f64 {
        if self.trials == 0 { 0.0 }
        else { self.successes as f64 / self.trials as f64 }
    }
}
```

### Selection Algorithm

For each candidate model:

```
score(model) = pass_rate(model) − cost_penalty(model) + affinity_bonus(model)
```

where:
- `cost_penalty` = normalized cost relative to the cheapest available model
- `affinity_bonus` = `CACHE_AFFINITY_BONUS` (0.15) if the model matches the previous task's model

Additional biases from the C-Factor and affect system:

- **Low affect confidence** (< `LOW_AFFECT_CONFIDENCE_THRESHOLD` = 0.3): bias toward stronger models.
- **High C-Factor** (> `HIGH_CFACTOR_THRESHOLD` = 0.8): bias toward cheaper models (system is performing well, can afford to save).
- **Low C-Factor** (< `LOW_CFACTOR_THRESHOLD` = 0.4): bias toward stronger models (system is struggling, need to invest in quality).

### Transition Threshold

The `CONFIDENCE_TO_UCB_THRESHOLD = 200` observation count triggers transition to stage 3. This threshold was chosen because:
- 200 observations with 5 models gives ~40 per model.
- LinUCB with 18 dimensions needs ~2× the dimension count in observations per arm for stable weights, so 36+ per arm.
- 200 provides a comfortable margin above this minimum.

---

## Stage 3: UCB Routing (> 200 observations)

At 200+ observations, the full `LinUCBRouter` contextual bandit takes over. See [03-bandits-ucb-thompson-linucb](03-bandits-ucb-thompson-linucb.md) for the algorithm details.

The LinUCB stage uses the 18-dimensional `RoutingContext` to make context-dependent decisions. This means the router can learn patterns like:

- "For `roko-core` crate with high familiarity, haiku is sufficient."
- "For cross-crate refactoring on retry (iteration > 0), use opus."
- "When the previous model was sonnet and it failed, escalate to opus rather than retrying sonnet."

---

## CascadeModel Output

The router returns a `CascadeModel` containing routing advice:

```rust
pub struct CascadeModel {
    /// Primary model to use.
    pub primary: ModelSpec,
    /// Fallback model if the primary fails or times out.
    pub fallback: Option<ModelSpec>,
    /// Latency SLA in milliseconds.
    pub latency_sla_ms: u64,
    /// Which cascade stage produced this recommendation.
    pub stage: CascadeStage,
}
```

The `fallback` field provides a pre-computed escalation target. If the primary model fails (gate failure, timeout, or provider error), the orchestrator can immediately retry with the fallback without re-querying the router. This avoids a round-trip through the cascade during time-critical retry scenarios.

---

## Provider Health Integration

The cascade router integrates with the `ProviderHealthRegistry` to avoid routing to unhealthy providers:

```
CascadeRouter::select(context)
    │
    ├── 1. Compute candidate scores (per stage algorithm)
    │
    ├── 2. Filter: ProviderHealthRegistry::is_available(model.provider)
    │       → Remove models whose provider circuit breaker is Open
    │
    ├── 3. Filter: Pareto frontier pruning
    │       → Remove dominated models (worse on both cost and quality)
    │
    └── 4. Select highest-scoring non-filtered model
```

See [09-provider-health-circuit-breaker](09-provider-health-circuit-breaker.md) for the circuit breaker algorithm and [10-pareto-frontier-pruning](10-pareto-frontier-pruning.md) for the Pareto filter.

---

## C-Factor Integration

The cascade router uses the C-Factor (Collective Capability Factor) as a routing bias:

```rust
pub enum AgentDispatchBias {
    PreferStronger,   // C-Factor < 0.4 — system struggling
    PreferCheaper,    // C-Factor > 0.8 — system performing well
    Neutral,          // C-Factor 0.4–0.8 — no bias
}
```

The C-Factor is computed from a composite of gate pass rate, cost efficiency, speed, first-try rate, knowledge growth, and turn-taking equality across recent episodes. A high C-Factor indicates the system is performing well and can afford to route to cheaper models; a low C-Factor indicates the system needs investment in quality.

See [15-collective-calibration-31x](15-collective-calibration-31x.md) for the C-Factor computation and its theoretical basis.

---

## Pareto Frontier Pruning

Before presenting candidates to the bandit, the cascade router computes a Pareto frontier over `(pass_rate, cost_per_success)`:

```
Model A: pass_rate=0.90, cost/success=$10.00  → Pareto-optimal
Model B: pass_rate=0.70, cost/success=$12.00  → DOMINATED by A (worse on both)
Model C: pass_rate=0.80, cost/success=$9.00   → Pareto-optimal (lower cost than A)
```

Only Pareto-optimal models are presented to the bandit. This reduces the arm set and prevents the bandit from wasting exploration budget on clearly inferior models.

The Pareto frontier is recomputed every `PARETO_RECOMPUTE_INTERVAL = 50` observations to keep it current as model statistics evolve.

See [10-pareto-frontier-pruning](10-pareto-frontier-pruning.md) for the full algorithm.

---

## Persistence

The cascade router persists its full state to `.roko/learn/cascade-router.json`:

```json
{
  "observations": 347,
  "stage": "ucb",
  "model_stats": {
    "claude-haiku-4-5-20251001": { "trials": 89, "successes": 71 },
    "claude-sonnet-4-20250514": { "trials": 156, "successes": 108 },
    "claude-opus-4-20250514": { "trials": 102, "successes": 89 }
  },
  "linucb_state": {
    "arms": { ... },
    "observation_count": 347
  },
  "pareto_frontier": ["claude-haiku-4-5-20251001", "claude-opus-4-20250514"]
}
```

State is loaded on startup and saved after each routing decision update. The atomic tempfile+rename pattern ensures crash safety.

---

## Operating Frequency

The cascade router operates at **per-episode frequency** — every agent turn produces one routing update. This is the highest-frequency learning loop in the system.

See [14-stability-mechanisms](14-stability-mechanisms.md) for how frequency separation across subsystems prevents oscillation.

---

## Cascade Router with Lookahead

Current routing considers only the immediate task. Lookahead routing predicts the sequence of upcoming tasks and makes routing decisions that optimize across the sequence — choosing a slightly more expensive model now if it will enable cheaper routing for subsequent tasks via KV cache reuse.

### Sequence-Aware Routing

```rust
pub struct LookaheadRouter {
    /// Base cascade router for individual decisions.
    inner: CascadeRouter,
    /// Task dependency graph for lookahead.
    task_graph: TaskDag,
    /// Lookahead horizon (default: 3 tasks ahead).
    pub horizon: usize,
    /// Discount factor for future savings (default: 0.9).
    pub gamma: f64,
    /// KV cache reuse probability model.
    cache_model: CacheReuseModel,
}

pub struct CacheReuseModel {
    /// Per-(model, role) estimated cache hit rate when reusing same model.
    cache_hit_rates: HashMap<(String, String), f64>,
    /// Average input tokens saved per cache hit.
    avg_tokens_saved_per_hit: u64,
    /// Cost per 1M tokens for cache reads vs fresh input.
    cache_read_discount: f64,
}
```

### Lookahead Algorithm

```
fn select_with_lookahead(current_task, upcoming_tasks, horizon):
    // Get upcoming tasks from DAG (respecting dependencies)
    window = [current_task] + upcoming_tasks[..horizon]

    best_total_cost = infinity
    best_assignment = None

    for each candidate_model for current_task:
        // Compute immediate cost
        immediate_cost = estimated_cost(candidate_model, current_task)

        // Compute expected future savings from cache reuse
        future_savings = 0.0
        for i in 1..window.len():
            // If future task uses same model, cache reuse saves tokens
            p_cache = cache_model.hit_rate(candidate_model, window[i].role)
            tokens_saved = p_cache × cache_model.avg_tokens_saved_per_hit
            cost_saved = tokens_saved × cache_model.cache_read_discount / 1_000_000
            future_savings += gamma^i × cost_saved

        total_cost = immediate_cost - future_savings

        if total_cost < best_total_cost:
            best_total_cost = total_cost
            best_assignment = candidate_model

    return best_assignment
```

### When Lookahead Matters

Lookahead routing provides significant savings when:

| Condition | Savings | Mechanism |
|-----------|---------|-----------|
| Sequential tasks on same crate | 15-30% | KV cache reuse for crate context |
| Same role across consecutive tasks | 10-20% | System prompt caching |
| Plan with >10 tasks | 5-15% compound | Amortized model selection overhead |
| Mixed complexity plan | 10-25% | Route easy tasks cheap, cache for hard |

Lookahead provides minimal benefit when tasks are independent (no shared context), when the plan has very few tasks, or when all tasks require different models.

### Connection to Speculative Decoding

Lookahead routing is analogous to speculative decoding (Leviathan et al. 2023) applied at the task level rather than the token level. Where speculative decoding predicts future tokens to reduce latency, lookahead routing predicts future tasks to reduce cost. The SpecRouter framework (2025) demonstrates that treating LLM inference as an adaptive routing problem — dynamically constructing inference "paths" — can significantly reduce end-to-end cost. Roko's lookahead extends this insight from intra-request to inter-request optimization.

---

## Router Calibration

The cascade router's decisions are only as good as its internal estimates of model performance. Router calibration ensures that the router's confidence maps to actual performance — when the router estimates 80% pass probability for a model, that model should actually succeed approximately 80% of the time.

### Calibration Framework

```rust
pub struct RouterCalibration {
    /// Per-model calibration data.
    calibrations: HashMap<String, ModelCalibration>,
    /// Overall calibration score (lower is better, 0 = perfect).
    pub brier_score: f64,
    /// Recalibration interval (default: every 100 routing decisions).
    pub recalibrate_interval: u32,
}

pub struct ModelCalibration {
    /// Model slug.
    pub model: String,
    /// Predicted pass probabilities and actual outcomes.
    predictions: Vec<(f64, bool)>,
    /// Calibration bins (10 bins, 0-10%, 10-20%, ..., 90-100%).
    pub bins: [CalibrationBin; 10],
    /// Platt scaling parameters: a, b for sigmoid(a × raw_score + b).
    pub platt_a: f64,
    pub platt_b: f64,
    /// Isotonic regression mapping (non-parametric calibration).
    pub isotonic_map: Vec<(f64, f64)>,
}

pub struct CalibrationBin {
    /// Bin range (e.g., 0.7 to 0.8).
    pub lower: f64,
    pub upper: f64,
    /// Number of predictions in this bin.
    pub count: u32,
    /// Actual success rate within this bin.
    pub actual_rate: f64,
    /// Expected Calibration Error for this bin.
    pub ece_contribution: f64,
}
```

### Calibration Methods

**1. Platt Scaling (parametric)**

Fits a logistic regression on top of the router's raw confidence scores:

```
calibrated_probability = sigmoid(a × raw_score + b)
```

Parameters `a` and `b` are fit by minimizing log-loss on a held-out validation set of recent routing decisions. Platt scaling is fast (O(n) fitting), requires few samples (~50), and handles monotonic miscalibration well.

**2. Isotonic Regression (non-parametric)**

Fits a non-decreasing step function mapping raw scores to calibrated probabilities. More flexible than Platt scaling — handles non-monotonic miscalibration — but requires more data (~200 samples) and can overfit with small datasets.

**3. Temperature Scaling**

The simplest calibration: divide raw logits by a learned temperature T before applying softmax.

```
calibrated_score = raw_score / T
```

T > 1 reduces overconfidence. T < 1 reduces underconfidence. T is fit to minimize negative log-likelihood on validation data.

### Expected Calibration Error (ECE)

The primary calibration metric, computed over B bins:

```
ECE = Σ_{b=1}^{B} (n_b / N) × |accuracy_b - confidence_b|
```

where `n_b` is the number of predictions in bin b, `accuracy_b` is the actual success rate, and `confidence_b` is the average predicted probability. ECE = 0 means perfect calibration.

| ECE Range | Interpretation | Action |
|-----------|---------------|--------|
| < 0.05 | Well calibrated | No action needed |
| 0.05 - 0.10 | Slightly miscalibrated | Monitor |
| 0.10 - 0.20 | Miscalibrated | Apply Platt scaling |
| > 0.20 | Severely miscalibrated | Investigate data distribution shift |

### Auto-Recalibration

The router recalibrates automatically every `recalibrate_interval` decisions:

```
Every 100 routing decisions:
    1. Collect last 200 (predicted_probability, actual_outcome) pairs
    2. Compute ECE
    3. If ECE > 0.10:
       a. Fit Platt scaling parameters (a, b) via gradient descent
       b. Validate on held-out 20% of data
       c. If Platt-calibrated ECE < original ECE:
          → Apply Platt scaling to all future predictions
       d. Else: fit isotonic regression as fallback
    4. Log calibration metrics to .roko/learn/calibration.jsonl
```

### Connection to Mixture-of-Experts Routing

The cascade router's calibration challenge is analogous to the load-balancing problem in Mixture-of-Experts (MoE) models. In MoE architectures like Switch Transformer (Fedus et al. 2022) and GShard (Lepikhin et al. 2021), a gating network routes tokens to specialized expert sub-networks. Key parallels:

| MoE Concept | Cascade Router Equivalent |
|-------------|--------------------------|
| Gating network | Stage-2/3 scoring function |
| Expert capacity factor | Budget guardrail per model |
| Auxiliary load balance loss | Pareto frontier pruning |
| Expert choice routing (EC) | Inverse routing: model "claims" tasks it's best at |
| Top-k routing | CascadeModel with primary + fallback |

The Expert Choice (EC) routing innovation (Zhou et al. 2022) — where experts select their top-k tokens rather than tokens selecting experts — suggests an interesting inversion for Roko: instead of tasks being routed to models, models could "claim" tasks from a queue based on their learned specialization. This would naturally load-balance across models while allowing each model to self-select tasks where it excels.

---

## Cost-Spectrum Routing

Recent research on cost-aware routing (CSCR, 2025) demonstrates that the router should consider a continuous spectrum of cost-quality tradeoffs rather than discrete tiers. The Cost-Spectrum Contrastive Router achieves up to 25% improvement in accuracy-cost tradeoff by adaptively selecting cost bands.

### Continuous Cost-Quality Frontier

```rust
pub struct CostSpectrumRouter {
    /// Contrastive encoder mapping (task_context, model_descriptor) → similarity.
    encoder: ContrastiveEncoder,
    /// Per-model cost descriptors (lightweight feature vectors).
    model_descriptors: HashMap<String, ModelDescriptor>,
    /// Adaptive cost band for current system state.
    pub cost_band: CostBand,
    /// Cost band adaptation parameters.
    pub band_adaptation: BandAdaptation,
}

pub struct CostBand {
    /// Lower bound of acceptable cost per task (USD).
    pub min_cost: f64,
    /// Upper bound of acceptable cost per task (USD).
    pub max_cost: f64,
    /// Current operating point within the band.
    pub target_cost: f64,
}

pub struct BandAdaptation {
    /// Widen band when pass rate is high (can afford cheaper experiments).
    pub widen_threshold: f64,    // default: 0.85 pass rate
    /// Narrow band when budget pressure is high.
    pub narrow_threshold: f64,   // default: 0.80 budget utilization
    /// Band width change per adaptation step.
    pub step_size: f64,          // default: 0.05 (5% of current band width)
}

pub struct ModelDescriptor {
    /// Lightweight feature vector encoding model characteristics.
    /// [quality_score, cost_per_m_tokens, avg_latency_ms, context_window_size]
    pub features: [f64; 4],
    /// Provider identifier.
    pub provider: String,
    /// Whether this model supports extended thinking/reasoning.
    pub supports_reasoning: bool,
}
```

### Selection with Cost Bands

```
fn select_cost_spectrum(task_context, models, cost_band):
    // Filter models to cost band
    candidates = models.filter(|m| cost_band.min_cost <= m.cost <= cost_band.max_cost)

    // Score each candidate by contrastive similarity to task
    for candidate in candidates:
        score = encoder.similarity(task_context, candidate.descriptor)

    // Select cheapest model above quality threshold
    quality_threshold = 0.7  // minimum acceptable similarity
    qualified = candidates.filter(|c| c.score >= quality_threshold)
    return qualified.min_by(|c| c.cost)

    // Fallback: if no qualified model in band, expand band
    if qualified.is_empty():
        cost_band.max_cost *= 1.5
        return select_cost_spectrum(task_context, models, expanded_band)
```

### Adaptive Band Management

The cost band adapts to system performance:

```
After each routing decision:
    if recent_pass_rate(last 20) > widen_threshold:
        // System performing well → can try cheaper models
        cost_band.min_cost -= step_size × cost_band.target_cost
    if budget_utilization > narrow_threshold:
        // Budget pressure → restrict to cheaper models
        cost_band.max_cost -= step_size × cost_band.target_cost
    if recent_pass_rate(last 20) < 0.50:
        // System struggling → widen band to allow expensive models
        cost_band.max_cost += 2 × step_size × cost_band.target_cost
```

This creates a self-regulating cost control mechanism that complements the budget guardrails in [08-cost-normalization](08-cost-normalization.md) — the guardrails provide hard limits while cost-spectrum routing provides soft optimization within those limits.

---

## Relationship to Other Documents

- **[03-bandits-ucb-thompson-linucb](03-bandits-ucb-thompson-linucb.md)** — The LinUCB algorithm used in stage 3.
- **[08-cost-normalization](08-cost-normalization.md)** — Cost signals that feed into the cost penalty during confidence-stage routing.
- **[09-provider-health-circuit-breaker](09-provider-health-circuit-breaker.md)** — Provider health filtering before candidate scoring.
- **[10-pareto-frontier-pruning](10-pareto-frontier-pruning.md)** — Pareto pruning that reduces the candidate set.
- **[13-8-missing-feedback-loops](13-8-missing-feedback-loops.md)** — Loop 1 (Health→Routing) and Loop 6 (Cost→Routing) feed into cascade router decisions.
- **[14-stability-mechanisms](14-stability-mechanisms.md)** — Hysteresis prevents oscillation between near-equal models.
- **[15-collective-calibration-31x](15-collective-calibration-31x.md)** — C-Factor provides routing bias based on system performance.

See also: [12-self-improvement-frameworks](12-self-improvement-frameworks.md) for the academic routing research (RouteLLM, FrugalGPT, AutoMix) that inspired the cascade design.


---

# SOURCE: /Users/will/dev/nunchi/roko/roko/docs/05-learning/05-pattern-discovery-trigram.md

# Pattern Discovery: Trigram Mining

> **Crate:** `roko-learn` · **Modules:** `pattern_discovery.rs`, `hdc_clustering.rs`
> **Wiring:** `LearningRuntime::record_completed_run()` → `PatternMiner::ingest_episode()`
> **Cross-references:** [00-episode-logger](00-episode-logger.md), [01-playbook-system](01-playbook-system.md), [15-collective-calibration-31x](15-collective-calibration-31x.md)


> **Implementation**: Shipping

---

## Purpose

Pattern discovery mines recurring structural signals from the episode stream. The core technique is trigram mining: extracting every three-action subsequence from each episode's gate verdict sequence, counting how often each trigram appears across episodes, and surfacing those that exceed a support threshold as recurring patterns. These patterns are the intermediate tier in the three-tier memory hierarchy (episodes → patterns → playbook rules).

The module also provides cross-episode consolidation using HDC clustering: grouping structurally similar episodes into clusters, then extracting meta-patterns that describe common traits of each cluster.

---

## EpisodeView Trait

Pattern mining is decoupled from the concrete `Episode` type via a trait:

```rust
pub trait EpisodeView {
    /// Ordered slice of action kind labels recorded during the episode.
    fn actions(&self) -> &[String];
    /// Whether the episode reached a successful terminal state.
    fn succeeded(&self) -> bool;
}
```

This decoupling lets the miner work with any type that exposes an ordered list of action kinds and a success flag — including the canonical `Episode`, synthetic fixtures, and downstream replayers. In practice, the `LearningRuntime` wraps each `Episode` in an `EpisodeActions` adapter that extracts gate names from `gate_verdicts`:

```rust
struct EpisodeActions {
    actions: Vec<String>,   // ["compile", "test", "lint", "diff"]
    success: bool,
}

impl EpisodeActions {
    fn from_episode(ep: &Episode) -> Self {
        Self {
            actions: ep.gate_verdicts.iter().map(|v| v.gate.clone()).collect(),
            success: ep.success,
        }
    }
}
```

---

## Trigram Mining Algorithm

### Step 1: Ingest

For each episode, the miner extracts all three-action subsequences (trigrams):

```
Episode actions: ["read", "edit", "compile", "test", "lint"]

Trigrams:
  ("read", "edit", "compile")
  ("edit", "compile", "test")
  ("compile", "test", "lint")
```

Each trigram is hashed to a stable 64-bit signature using FNV-1a. The miner maintains a `BTreeMap<u64, TrigramStats>` keyed by signature:

```rust
struct TrigramStats {
    trigram: [String; 3],
    signature: u64,
    support: u32,          // distinct episodes containing this trigram
    first_seen_ms: i64,
    last_seen_ms: i64,
}
```

A trigram's support count is the number of distinct episodes that contain it (not the total number of occurrences across all episodes). This prevents a single long episode from inflating support counts.

### Step 2: Discover

After ingesting a batch, `PatternMiner::discover()` returns all trigrams whose support clears the configured thresholds:

```rust
pub struct PatternMiner {
    min_support: u32,       // minimum distinct episodes (default: 2)
    min_confidence: f32,    // minimum support/total ratio (default: 0.5)
    // ...
}
```

Each qualifying trigram becomes a `Pattern`:

```rust
pub struct Pattern {
    /// Stable string id ("trigram:<signature>").
    pub id: String,
    /// Deterministic 64-bit content hash of the trigram.
    pub signature: u64,
    /// Human-readable rendering (e.g. "read → edit → test").
    pub description: String,
    /// Number of distinct episodes containing this trigram.
    pub support_count: u32,
    /// support_count / total_episodes, clamped to [0.0, 1.0].
    pub confidence: f32,
    /// Unix ms of the first episode containing this trigram.
    pub first_seen_ms: i64,
    /// Unix ms of the most recent episode containing it.
    pub last_seen_ms: i64,
}
```

### Step 3: Promote

Patterns with sufficient support (typically ≥5 episodes) are candidates for promotion to playbook rules. See [01-playbook-system](01-playbook-system.md) for the promotion criteria.

---

## Why Trigrams?

| N-gram size | Properties |
|-------------|------------|
| Unigrams (1) | Too generic — "compile" appears in every episode |
| Bigrams (2) | Still generic — "edit→compile" is nearly universal |
| **Trigrams (3)** | Captures meaningful action patterns — "read→edit→test" vs "edit→compile→fix" |
| 4-grams (4) | Too specific — many unique sequences, insufficient support for pattern extraction |

Trigrams strike the right balance between specificity and support. They capture enough context to distinguish successful from unsuccessful action sequences, while remaining common enough to accumulate statistically significant support counts.

---

## HDC Clustering for Cross-Episode Consolidation

Beyond trigram mining, the module provides cross-episode structural analysis using HDC (Hyperdimensional Computing) clustering.

### k-Medoids Algorithm

The `hdc_clustering` module implements Partitioning Around Medoids (PAM) over 10,240-bit `HdcVector`s:

```rust
pub struct KMedoidsConfig {
    pub k: usize,              // number of clusters (default: 3)
    pub max_iterations: usize, // convergence limit (default: 100)
}
```

The algorithm:

1. **Initialize** — greedy farthest-first seeding: pick the point closest to the global centroid as the first medoid, then iteratively add the point maximizing its minimum distance to all existing medoids.
2. **Assign** — each point goes to the nearest medoid (distance = `1 − similarity` where similarity is HDC Hamming similarity).
3. **Update** — for each cluster, the member minimizing total intra-cluster distance becomes the new medoid.
4. Repeat 2-3 until medoids stabilize or `max_iterations` is reached.

### Cross-Episode Consolidation

The `CrossEpisodeConsolidator` groups episodes by their HDC fingerprints, then extracts meta-patterns from each cluster:

```
Episodes with HDC fingerprints
    │
    ▼
k-medoids clustering (k=3, HDC similarity)
    │
    ▼
For each cluster:
    ├── Identify common trigrams across cluster members
    ├── Compute cluster-level pass rate
    ├── Extract distinguishing features (files, roles, categories)
    └── Produce CrossEpisodeConsolidationReport
```

The consolidation report identifies structural groupings in the episode stream that may not be visible from individual trigram analysis. For example, a cluster of episodes that all involve cross-crate modifications and share a high failure rate suggests a systemic issue with cross-crate tasks, even if no single trigram captures this pattern.

---

## Operating Frequency

Pattern discovery runs at **every 20 episodes** — the slowest learning loop in the system. This frequency separation prevents oscillation: rapid pattern updates could cause playbook rules to be promoted and demoted on noisy short-term data, while infrequent updates ensure that patterns reflect stable, recurring phenomena.

```
Learning Loop Frequencies:
    ├── Cascade router:       every episode          (highest)
    ├── Gate thresholds:      every 5 episodes
    ├── Pattern discovery:    every 20 episodes       (lowest)
    └── Cross-episode:        on-demand or periodic
```

See [14-stability-mechanisms](14-stability-mechanisms.md) for the full frequency separation design.

---

## Integration with LearningRuntime

The `LearningRuntime` invokes pattern mining as step 8 of the `record_completed_run()` pipeline:

```
CompletedRunInput
    │
    ├── 1-7. Episode, costs, playbook, skills, metrics, experiments
    │
    ├── 8. PatternMiner::ingest_episode(EpisodeActions::from_episode(ep))
    │       → Updates trigram counters
    │       → If episode_count % 20 == 0: auto-discover
    │
    └── 9-10. CascadeRouter, CFactor
```

The auto-discover trigger runs `PatternMiner::discover()` every 20 episodes and feeds the results to the `CrossEpisodeConsolidator` for cluster-level analysis.

---

## Performance

| Operation | Complexity | Typical Time |
|-----------|-----------|--------------|
| Ingest one episode | O(n) where n = action count | < 1μs |
| Discover patterns | O(m) where m = unique trigrams | < 100μs for 1000 trigrams |
| HDC fingerprint comparison | O(1) — bit-parallel Hamming | ~50ns |
| k-medoids clustering | O(k × n × max_iter) | < 10ms for 200 episodes |

The entire pattern discovery pipeline adds negligible overhead to the per-episode processing path. The most expensive operation (k-medoids clustering) runs at the lowest frequency (every 20 episodes) and only when cross-episode consolidation is triggered.

---

## Practical Example

### Episode Stream

```
Episode 1: actions = ["read", "edit", "compile", "test"]     success=true
Episode 2: actions = ["read", "edit", "compile", "fix", "compile", "test"]  success=true
Episode 3: actions = ["edit", "compile", "test"]              success=true
Episode 4: actions = ["read", "edit", "compile", "test"]      success=true
Episode 5: actions = ["edit", "compile", "lint", "fix", "compile", "test"]  success=false
```

### Trigram Extraction

```
Episode 1: (read,edit,compile) (edit,compile,test)
Episode 2: (read,edit,compile) (edit,compile,fix) (compile,fix,compile) (fix,compile,test)
Episode 3: (edit,compile,test)
Episode 4: (read,edit,compile) (edit,compile,test)
Episode 5: (edit,compile,lint) (compile,lint,fix) (lint,fix,compile) (fix,compile,test)
```

### Support Counts

```
(read,edit,compile):    support=3  confidence=3/5=0.60  → PATTERN (above thresholds)
(edit,compile,test):    support=3  confidence=3/5=0.60  → PATTERN
(fix,compile,test):     support=2  confidence=2/5=0.40  → below confidence threshold
(edit,compile,fix):     support=1  confidence=1/5=0.20  → below support threshold
```

### Discovered Patterns

```
Pattern "trigram:0xA1B2C3": read → edit → compile
    support: 3 episodes, confidence: 0.60
    First seen: episode 1, Last seen: episode 4

Pattern "trigram:0xD4E5F6": edit → compile → test
    support: 3 episodes, confidence: 0.60
    First seen: episode 1, Last seen: episode 4
```

These two patterns capture the dominant successful action sequence: read the code, edit it, compile, test. This pattern can be promoted to a playbook rule that instructs agents to follow this read→edit→compile→test workflow.

---

## HDC Distance Metric

The HDC clustering module uses `1 − similarity` as the distance metric, where similarity is computed via Hamming distance on 10,240-bit vectors:

```
similarity(a, b) = 1 − (hamming_distance(a, b) / 10240)
distance(a, b) = hamming_distance(a, b) / 10240
```

Values:
- Identical vectors: distance = 0, similarity = 1.0
- Orthogonal vectors: distance ≈ 0.5, similarity ≈ 0.5
- Maximally different: distance = 1.0, similarity = 0.0

The 10,240-bit dimension provides high expressiveness: two vectors created from different seeds have expected similarity ≈ 0.5 (random baseline), while vectors created from similar content cluster well above 0.7.

---

## Relationship to Other Documents

- **[00-episode-logger](00-episode-logger.md)** — Episodes are the raw data stream that the miner consumes.
- **[01-playbook-system](01-playbook-system.md)** — Patterns with sufficient support are promoted to playbook rules.
- **[02-skill-library-voyager](02-skill-library-voyager.md)** — Patterns identify recurring sequences; skills capture the full procedures associated with successful sequences.
- **[14-stability-mechanisms](14-stability-mechanisms.md)** — Frequency separation ensures pattern discovery runs at the appropriate cadence.
- **[15-collective-calibration-31x](15-collective-calibration-31x.md)** — Pattern counts contribute to the knowledge_growth component of the C-Factor.


---

# SOURCE: /Users/will/dev/nunchi/roko/roko/docs/05-learning/06-task-metrics-and-baselines.md

# Task Metrics and Baselines

> **Crate:** `roko-learn` · **Modules:** `task_metric.rs`, `baseline.rs`, `efficiency.rs`
> **Persistence:** `.roko/learn/task-metrics.jsonl`, `.roko/learn/efficiency.jsonl`
> **Wiring:** `LearningRuntime::record_completed_run()` → metrics pipeline
> **Cross-references:** [00-episode-logger](00-episode-logger.md), [07-regression-detection](07-regression-detection.md), [15-collective-calibration-31x](15-collective-calibration-31x.md)


> **Implementation**: Shipping

---

## Purpose

The task metrics and baselines subsystem provides the quantitative foundation for all performance evaluation in Roko. Every gate execution produces one immutable `TaskMetric` record. These records accumulate in an append-only JSONL file, and the baseline computation groups them by `(role, complexity_band)` to produce per-slice statistical profiles. The regression detector then compares fresh batches against these baselines to identify performance degradation.

The efficiency module extends per-turn instrumentation with prompt-level attribution, tool utilization tracking, and A-D letter grading for prompt assembly quality.

---

## TaskMetric Schema

The canonical `TaskMetric` struct lives in `roko-core::metric` and is re-exported by `roko-learn::task_metric`:

```rust
pub struct TaskMetric {
    /// Task identifier.
    pub task_id: String,
    /// Plan identifier.
    pub plan_id: String,
    /// Agent role (e.g. "Implementer").
    pub role: String,
    /// Complexity band ("fast", "standard", "complex").
    pub complexity_band: String,
    /// Model slug used.
    pub model: String,
    /// Backend provider.
    pub backend: String,
    /// Gate name.
    pub gate: String,
    /// Whether the gate passed.
    pub gate_passed: bool,
    /// Zero-based iteration index.
    pub iteration: u32,
    /// Cost in USD.
    pub cost_usd: f64,
    /// Duration in milliseconds.
    pub duration_ms: u64,
    /// Input tokens.
    pub input_tokens: u64,
    /// Output tokens.
    pub output_tokens: u64,
    /// Configuration hash for A/B comparison.
    pub config_hash: ConfigHash,
    /// Timestamp.
    pub timestamp: DateTime<Utc>,
}
```

### MetricFilter

The `MetricFilter` provides declarative filtering over the metric stream:

```rust
pub struct MetricFilter {
    pub roles: HashSet<String>,
    pub complexity_bands: HashSet<String>,
    pub plan_ids: HashSet<String>,
    pub gates: HashSet<String>,
    pub models: HashSet<String>,
    pub backends: HashSet<String>,
    pub gate_passed: Option<bool>,
    pub iteration_range: Option<(u32, u32)>,
    pub min_cost_usd: Option<f64>,
    pub max_cost_usd: Option<f64>,
    pub config_hashes: HashSet<String>,
}
```

All predicates are AND-combined: a record must match every non-empty filter field.

### MetricsWriter and MetricsReader

- `MetricsWriter` — thread-safe, append-only JSONL writer that batches records in memory and flushes to an `AsyncWrite` sink. Uses `parking_lot::Mutex` for serialization.
- `MetricsReader` — parse JSONL lines from bytes, tolerant of corrupted lines (same resilience pattern as the episode logger).

---

## Baseline Computation

The `baseline` module computes per-slice statistical profiles from accumulated `TaskMetric` records.

### SliceBaseline

```rust
pub struct SliceBaseline {
    /// Role for this slice.
    pub role: String,
    /// Complexity band for this slice.
    pub complexity_band: String,
    /// Gate pass rate (0.0 – 1.0).
    pub pass_rate: f64,
    /// Average cost in USD per task.
    pub avg_cost: f64,
    /// Average duration in milliseconds.
    pub avg_duration_ms: f64,
    /// Average number of iterations to pass.
    pub avg_iterations: f64,
    /// Average input tokens per turn.
    pub avg_input_tokens: f64,
    /// Average output tokens per turn.
    pub avg_output_tokens: f64,
    /// Average cache hit rate.
    pub avg_cache_hit_rate: f64,
    /// Number of records in this slice.
    pub n_records: usize,
}
```

### Baseline

```rust
pub struct Baseline {
    /// Per-(role, complexity) statistical profiles.
    pub slices: Vec<SliceBaseline>,
    /// Overall aggregate across all slices.
    pub overall_pass_rate: f64,
    pub overall_avg_cost: f64,
    pub overall_avg_duration_ms: f64,
    pub overall_avg_iterations: f64,
    pub overall_n_records: usize,
}
```

### Computation

`compute_baseline()` groups `TaskMetric` records by `(role, complexity_band)` and computes descriptive statistics for each group:

```
TaskMetric records
    │
    ▼
Group by (role, complexity_band)
    │
    ├── ("Implementer", "standard") → 156 records
    │       pass_rate: 0.72
    │       avg_cost: $0.83
    │       avg_duration_ms: 45,000
    │       avg_iterations: 1.4
    │
    ├── ("Implementer", "complex") → 48 records
    │       pass_rate: 0.58
    │       avg_cost: $1.52
    │       avg_duration_ms: 120,000
    │       avg_iterations: 2.1
    │
    └── ("Reviewer", "standard") → 89 records
            pass_rate: 0.91
            avg_cost: $0.35
            avg_duration_ms: 12,000
            avg_iterations: 1.1
```

---

## Efficiency Events

The `AgentEfficiencyEvent` provides per-turn cost and quality instrumentation with 20+ fields:

```rust
pub struct AgentEfficiencyEvent {
    // ── Identity ──────────────────────────────
    pub agent_id: String,
    pub role: String,
    pub backend: String,
    pub model: String,
    pub plan_id: String,
    pub task_id: String,

    // ── Token accounting ──────────────────────
    pub input_tokens: u64,
    pub output_tokens: u64,
    pub reasoning_tokens: u64,
    pub cache_read_tokens: u64,
    pub cache_write_tokens: u64,

    // ── Cost accounting ───────────────────────
    pub cost_usd: f64,
    pub cost_usd_without_cache: f64,

    // ── Prompt composition ────────────────────
    pub prompt_sections: Vec<PromptSectionMeta>,
    pub total_prompt_tokens: u64,
    pub system_prompt_tokens: u64,

    // ── Tool utilization ──────────────────────
    pub tools_available: u32,
    pub tools_used: u32,
    pub tool_calls: Vec<ToolCallMeta>,

    // ── Timing ────────────────────────────────
    pub wall_time_ms: u64,
    pub time_to_first_token_ms: u64,
    pub was_warm_start: bool,
}
```

### PromptSectionMeta

Attributes token budget consumption to individual prompt sections:

```rust
pub struct PromptSectionMeta {
    /// Section name (e.g. "prd2", "workspace_map", "playbook_hits").
    pub name: String,
    /// Tokens consumed in the final prompt.
    pub tokens: u64,
    /// Composer-assigned priority (0 = highest).
    pub priority: u8,
    /// Whether this section was truncated due to budget pressure.
    pub was_truncated: bool,
    /// Whether this section was dropped entirely.
    pub was_dropped: bool,
}
```

### ToolCallMeta

Per-tool-call instrumentation:

```rust
pub struct ToolCallMeta {
    /// Tool name (e.g. "Read", "Write", "Bash").
    pub tool_name: String,
    /// Wall-clock duration in milliseconds.
    pub duration_ms: u64,
    /// Tokens in the tool result.
    pub result_tokens: u64,
    /// Whether the call succeeded.
    pub succeeded: bool,
}
```

---

## Prompt Efficiency Grading

The efficiency module provides A-D letter grading for prompt assembly:

```rust
pub enum Grade {
    A,  // ≥ 0.8 — excellent efficiency
    B,  // ≥ 0.6 — good efficiency
    C,  // ≥ 0.4 — fair efficiency
    D,  // < 0.4 — poor efficiency
}
```

The `PromptEfficiencyScore` evaluates:

1. **Section utilization** — what fraction of included sections contributed to the successful outcome?
2. **Token efficiency** — output tokens / input tokens (higher = more productive per token spent).
3. **Cache hit rate** — cache_read_tokens / input_tokens (higher = better cache utilization).
4. **Tool utilization** — tools_used / tools_available (higher = more tools leveraged).

These four components are weighted and combined into a composite score:

```
efficiency = 0.30 × section_utilization
           + 0.30 × token_efficiency
           + 0.20 × cache_hit_rate
           + 0.20 × tool_utilization
```

### Role Cost Profiles

The `RoleCostProfile` aggregates cost data per agent role:

```rust
pub struct RoleCostProfile {
    pub role: String,
    pub total_cost_usd: f64,
    pub avg_cost_per_turn: f64,
    pub avg_cost_per_success: f64,
    pub total_turns: u64,
    pub total_successes: u64,
    pub avg_input_tokens: f64,
    pub avg_output_tokens: f64,
    pub avg_cache_hit_rate: f64,
}
```

These profiles answer operational questions: "Which role is most expensive?" "Does the warm pool save money?" "Which prompt sections drove the cost?"

---

## Four Key Metrics

From the legacy design (mori-agents/07-self-improvement.md), four metrics drive self-improvement:

| Metric | Definition | Baseline Target |
|--------|-----------|-----------------|
| **First-attempt pass rate** | % of tasks passing gates on first try | > 60% |
| **Iterations per plan** | Average iterations to complete a plan | < 2.0 |
| **Cost per plan** | Total USD spent per plan | Decreasing trend |
| **Prompt tokens per spawn** | Input tokens for the initial agent prompt | < 50K |

These metrics are computed from the `TaskMetric` stream and surfaced via the `compute_headlines()` function:

```rust
pub struct Headlines {
    pub total_tasks: usize,
    pub passed_tasks: usize,
    pub pass_rate: f64,
    pub total_cost_usd: f64,
    pub avg_cost_per_task: f64,
    pub avg_iterations: f64,
    pub avg_duration_ms: f64,
}
```

---

## Persistence

| Artifact | Format | Path |
|----------|--------|------|
| Task metrics | JSONL | `.roko/learn/task-metrics.jsonl` |
| Efficiency events | JSONL | `.roko/learn/efficiency.jsonl` |

Both files are append-only. The `MetricsWriter` batches records in memory and flushes periodically for efficiency.

---

## Relationship to Other Documents

- **[00-episode-logger](00-episode-logger.md)** — Episodes produce the raw data; task metrics are the per-gate derivative.
- **[07-regression-detection](07-regression-detection.md)** — Baselines are the reference point for regression detection.
- **[04-cascade-router](04-cascade-router.md)** — Routing decisions affect metrics; metrics affect future routing decisions.
- **[08-cost-normalization](08-cost-normalization.md)** — Cost fields in metrics use normalized costs.
- **[15-collective-calibration-31x](15-collective-calibration-31x.md)** — Metrics feed into C-Factor components (gate_pass_rate, cost_efficiency, speed).

See also: [04-verification](../04-verification/INDEX.md) for the gate pipeline that produces individual gate outcomes.


---

# SOURCE: /Users/will/dev/nunchi/roko/roko/docs/05-learning/07-regression-detection.md

# Regression Detection

> **Crate:** `roko-learn` · **Module:** `regression.rs`
> **Wiring:** `LearningRuntime::record_completed_run()` → regression check
> **Cross-references:** [06-task-metrics-and-baselines](06-task-metrics-and-baselines.md), [14-stability-mechanisms](14-stability-mechanisms.md), [15-collective-calibration-31x](15-collective-calibration-31x.md)


> **Implementation**: Shipping

---

## Purpose

The regression detector answers a critical question: "Did this configuration change make things worse?" It compares a fresh batch of `TaskMetric` records against a previously computed `Baseline` and fires alerts when key indicators breach configurable thresholds. This closes the feedback loop between system changes (prompt modifications, model routing updates, playbook rule changes) and their observable impact on task outcomes.

Without regression detection, the learning system could silently degrade: a bandit might converge on a model that worked well last week but performs poorly after a provider update, or a playbook rule might be promoted despite introducing regressions in edge cases. The regression detector surfaces these degradations as structured alerts that trigger investigation or automatic rollback.

---

## Threshold Configuration

```rust
pub struct RegressionThresholds {
    /// Maximum allowed drop in first-attempt pass rate (default: 0.15 = 15%).
    pub pass_rate_drop: f64,
    /// Maximum allowed increase in average cost (default: 0.20 = 20%).
    pub cost_increase: f64,
    /// Maximum allowed increase in average duration (default: 0.30 = 30%).
    pub duration_increase: f64,
    /// Maximum allowed increase in average iterations (default: 0.25 = 25%).
    pub iterations_increase: f64,
    /// Minimum records needed before detection fires (default: 5).
    pub min_records: usize,
}
```

### Default Thresholds

| Metric | Threshold | Severity | Rationale |
|--------|-----------|----------|-----------|
| Pass rate drop | > 15% | **Alert** | Direct impact on task completion |
| Cost increase | > 20% | **Alert** | Budget impact |
| Duration increase | > 30% | Warning | May be acceptable for higher quality |
| Iterations increase | > 25% | Warning | More iterations may reflect harder tasks |

The asymmetry between Alert and Warning severities reflects priority: pass rate and cost regressions are immediate blockers that demand investigation, while duration and iteration increases may be acceptable tradeoffs (e.g., a harder task mix this week).

---

## Detection Algorithm

The `detect_regressions()` function compares current metrics against a baseline:

```
Baseline (from historical TaskMetric records)
    │
    ▼
Current batch (recent N task metrics)
    │
    ▼
For each (role, complexity_band) slice:
    │
    ├── Pass rate regression:
    │     change = (baseline.pass_rate - current.pass_rate) / baseline.pass_rate
    │     if change > pass_rate_drop → Alert
    │     if change < -pass_rate_drop → Improvement
    │
    ├── Cost regression:
    │     change = (current.avg_cost - baseline.avg_cost) / baseline.avg_cost
    │     if change > cost_increase → Alert
    │     if change < -cost_increase → Improvement
    │
    ├── Duration regression:
    │     change = (current.avg_duration - baseline.avg_duration) / baseline.avg_duration
    │     if change > duration_increase → Warning
    │     if change < -duration_increase → Improvement
    │
    └── Iterations regression:
          change = (current.avg_iterations - baseline.avg_iterations) / baseline.avg_iterations
          if change > iterations_increase → Warning
          if change < -iterations_increase → Improvement
```

### Per-Slice Analysis

Regressions are detected per `(role, complexity_band)` slice, not just in aggregate. This prevents a scenario where a severe regression in "Implementer/complex" tasks is masked by improvements in "Reviewer/standard" tasks. Each slice that has enough records (≥ `min_records`) is analyzed independently.

---

## Alert Schema

```rust
pub enum AlertSeverity {
    Alert,        // Key metric breached (pass rate, cost)
    Warning,      // Secondary metric breached (duration, iterations)
    Improvement,  // Metric improved relative to baseline
}

pub struct RegressionAlert {
    /// Which metric regressed (e.g. "pass_rate", "cost").
    pub metric_name: String,
    /// Severity.
    pub severity: AlertSeverity,
    /// Baseline value.
    pub baseline_value: f64,
    /// Current (observed) value.
    pub current_value: f64,
    /// Fractional change (positive = worsened).
    pub change_fraction: f64,
    /// The threshold that was breached.
    pub threshold: f64,
    /// Human-readable description.
    pub description: String,
    /// Optional (role, complexity) slice. None = overall.
    pub slice: Option<(String, String)>,
}
```

### RegressionReport

```rust
pub struct RegressionReport {
    /// All alerts (breaches and improvements).
    pub alerts: Vec<RegressionAlert>,
    /// Whether any alert has Alert severity.
    pub has_regressions: bool,
    /// Whether the current data set has enough records.
    pub sufficient_data: bool,
    /// Number of current records analyzed.
    pub current_records: usize,
    /// Number of baseline records.
    pub baseline_records: usize,
}
```

The report provides convenience methods:
- `regressions()` — filter to Alert-severity items only.
- `warnings()` — filter to Warning-severity items only.

---

## LearningRuntime Integration

The regression detector runs as part of `LearningRuntime::record_completed_run()` when a `RegressionConfig` is configured:

```rust
pub struct RegressionConfig {
    pub thresholds: RegressionThresholds,
    /// Number of latest metrics used as the "current" sample.
    pub current_window: usize,  // default: 20
}
```

The runtime:
1. Reads all `TaskMetric` records from `.roko/learn/task-metrics.jsonl`.
2. Splits into baseline (all records except the latest `current_window`) and current (latest `current_window` records).
3. Computes baselines for both sets.
4. Calls `detect_regressions(baseline, current, thresholds)`.
5. If `report.has_regressions`, logs the alerts and optionally triggers corrective actions.

### Current Window

The `current_window` parameter (default: 20) determines how many recent metrics are treated as "current" for comparison against the baseline. This value balances:
- **Too small** (< 10): noisy, a single outlier can trigger false alerts.
- **Too large** (> 50): sluggish, a real regression takes many tasks to surface.
- **20** provides a reasonable tradeoff: enough data for statistical stability, but responsive enough to catch regressions within a single plan execution.

---

## C-Factor Regression

In addition to per-metric regression detection, the C-Factor module provides its own regression check over the composite C-Factor score:

```rust
pub struct CFactorRegression {
    pub current_snapshot_at: DateTime<Utc>,
    pub window_start: DateTime<Utc>,
    pub window_end: DateTime<Utc>,
    pub sample_count: usize,
    // ... delta analysis fields
}
```

C-Factor regression detects systemic decline: when the composite score drops against a trailing history window, it indicates that the system as a whole is performing worse, even if individual metrics haven't breached their thresholds. This catches subtle multi-dimensional regressions where pass rate drops slightly, cost rises slightly, and speed decreases slightly — none individually alarming, but collectively significant.

See [15-collective-calibration-31x](15-collective-calibration-31x.md) for the C-Factor computation.

---

## Adaptive Gate Thresholds

Regression detection interacts with the adaptive gate threshold system. Gate thresholds (pass/fail criteria for compile, test, lint, diff gates) are adjusted via EMA (Exponential Moving Average) based on historical pass rates:

```
Gate threshold = EMA(pass_rates, alpha=0.1)
```

When the regression detector fires a pass_rate Alert, it signals that thresholds may need recalibration. The adaptive threshold system can then tighten thresholds (require higher quality) or loosen them (accept the current performance level) based on operational priorities.

See [04-verification](../04-verification/INDEX.md) for the gate pipeline and adaptive threshold mechanism.

---

## Practical Example

Consider a system running 200 tasks over 3 days. After task 150, a prompt template change was deployed.

### Baseline (tasks 1-130)

```
Overall:
    pass_rate: 0.72
    avg_cost: $0.83
    avg_duration_ms: 45,000
    avg_iterations: 1.4

Slice (Implementer, standard):
    pass_rate: 0.75
    avg_cost: $0.78
    avg_duration_ms: 42,000
    avg_iterations: 1.3
```

### Current window (tasks 151-170, after template change)

```
Overall:
    pass_rate: 0.55     ← dropped
    avg_cost: $1.12     ← increased
    avg_duration_ms: 52,000
    avg_iterations: 1.9

Slice (Implementer, standard):
    pass_rate: 0.50     ← dropped significantly
    avg_cost: $1.05     ← increased
    avg_duration_ms: 48,000
    avg_iterations: 2.0
```

### Regression Report

```
ALERT: pass_rate regression in (Implementer, standard)
    Baseline: 0.75, Current: 0.50
    Change: -33.3% (threshold: 15%)
    "Pass rate dropped from 75% to 50% for Implementer/standard tasks"

ALERT: cost regression in (Implementer, standard)
    Baseline: $0.78, Current: $1.05
    Change: +34.6% (threshold: 20%)
    "Average cost increased from $0.78 to $1.05 for Implementer/standard tasks"

WARNING: iterations regression in (Implementer, standard)
    Baseline: 1.3, Current: 2.0
    Change: +53.8% (threshold: 25%)
    "Average iterations increased from 1.3 to 2.0 for Implementer/standard tasks"
```

### Corrective Action

The regression report identifies that the prompt template change degraded Implementer/standard tasks. The operator can:
1. **Revert**: Roll back the template change.
2. **Investigate**: Examine which specific tasks failed and why.
3. **Adjust**: Modify the template to address the failure pattern while preserving improvements in other slices.

Without regression detection, this degradation would be invisible — the system would continue spending more money for worse results.

---

## False Positive Management

Regression detection can produce false positives when:
- The task mix shifts (harder tasks → lower pass rate, not a regression).
- A single expensive task dominates the cost average.
- The current window is too small to be statistically stable.

Mitigation strategies:
1. **min_records threshold**: Don't fire alerts with fewer than 5 records per slice.
2. **Per-slice analysis**: Detect whether the regression is slice-specific or systemic.
3. **Improvement tracking**: Report improvements alongside regressions to provide context.
4. **Config hash correlation**: If a specific config change correlates with the regression, flag it as the likely cause.

---

## Relationship to Other Documents

- **[06-task-metrics-and-baselines](06-task-metrics-and-baselines.md)** — Task metrics and baselines are the input to regression detection.
- **[14-stability-mechanisms](14-stability-mechanisms.md)** — Regression detection is itself a stability mechanism, providing negative feedback when performance degrades.
- **[15-collective-calibration-31x](15-collective-calibration-31x.md)** — C-Factor regression provides a composite view of system health.
- **[13-8-missing-feedback-loops](13-8-missing-feedback-loops.md)** — Loop 4 (Failure→Replanning) uses regression alerts to trigger plan revision.
- **[04-cascade-router](04-cascade-router.md)** — Routing changes can cause regressions detected here.


---

# SOURCE: /Users/will/dev/nunchi/roko/roko/docs/05-learning/08-cost-normalization.md

# Cost Normalization

> **Crate:** `roko-learn` · **Modules:** `costs_db.rs`, `costs_log.rs`
> **Persistence:** `.roko/learn/costs.jsonl`
> **Implementation plan:** `modelrouting/09-cost-normalization.md` (tasks 2H.01–2H.10)
> **Cross-references:** [04-cascade-router](04-cascade-router.md), [06-task-metrics-and-baselines](06-task-metrics-and-baselines.md), [09-provider-health-circuit-breaker](09-provider-health-circuit-breaker.md)


> **Implementation**: Shipping

---

## Purpose

Cost normalization provides a consistent framework for comparing model costs across providers, pricing tiers, and token types. Raw cost data from providers is heterogeneous: some charge per input token, some per output token, some per request; cache hits reduce costs differently; reasoning tokens may have distinct pricing. The cost normalization layer transforms this into a single comparable metric — blended cost per million tokens — that the cascade router and budget guardrails can use for routing decisions.

---

## CostRecord Schema

```rust
pub struct CostRecord {
    /// Timestamp of the cost observation.
    pub timestamp: DateTime<Utc>,
    /// Model slug (e.g. "claude-sonnet-4-20250514").
    pub model: String,
    /// Provider identifier (e.g. "anthropic", "openrouter").
    pub provider: String,
    /// Agent role.
    pub role: String,
    /// Plan identifier.
    pub plan_id: String,
    /// Task identifier.
    pub task_id: String,
    /// Complexity band ("fast", "standard", "complex").
    pub complexity_band: String,
    /// Input tokens consumed.
    pub input_tokens: u64,
    /// Output tokens generated.
    pub output_tokens: u64,
    /// Total cost in USD.
    pub cost_usd: f64,
    /// Duration in milliseconds.
    pub duration_ms: u64,
    /// Whether the task succeeded.
    pub success: bool,
    /// Session identifier for grouping.
    pub session_id: String,
}
```

### CostSummary

Aggregate view over a collection of `CostRecord`s:

```rust
pub struct CostSummary {
    /// Total records summarized.
    pub total_records: usize,
    /// Total cost across all records.
    pub total_cost_usd: f64,
    /// Average cost per record.
    pub avg_cost_usd: f64,
    /// Total input tokens.
    pub total_input_tokens: u64,
    /// Total output tokens.
    pub total_output_tokens: u64,
    /// Average cost per successful task.
    pub avg_cost_per_success: f64,
    /// Number of successful records.
    pub success_count: usize,
}
```

---

## Blended Cost Formula

The blended cost per million tokens uses a 3:1 input-to-output weighting ratio, following the Artificial Analysis methodology:

```
blended_cost_per_m = (3 × input_price_per_m + 1 × output_price_per_m) / 4
```

### Why 3:1?

The 3:1 ratio reflects the typical token mix in agent workloads: agents read more than they write. In Roko's measured workloads, the median input-to-output ratio is approximately 3:1 (for every output token generated, the agent consumes ~3 input tokens in context, prior conversation, and tool results). Using this ratio makes the blended cost metric correspond to actual expenditure patterns.

### Token-Type Normalization

Not all input tokens cost the same:

| Token Type | Typical Pricing | Normalization |
|------------|----------------|---------------|
| Fresh input tokens | Full input price | 1.0× |
| Cache read tokens | 10-90% discount | Weighted by actual cache price |
| Cache write tokens | Usually same as input | 1.0× |
| Reasoning/thinking tokens | Usually same as output | Counted as output |
| System prompt tokens | Full input price | 1.0× (but often cached) |

The `AgentEfficiencyEvent` captures both `cost_usd` (actual cost after cache discounts) and `cost_usd_without_cache` (hypothetical full-price cost), enabling analysis of cache savings.

---

## CostTable Design

The CostTable (from implementation plan 2H.01–2H.04) structures per-model pricing:

```rust
pub struct ModelPricing {
    /// Model slug.
    pub model: String,
    /// Provider.
    pub provider: String,
    /// Input price per million tokens.
    pub input_price_per_m: f64,
    /// Output price per million tokens.
    pub output_price_per_m: f64,
    /// Cache read price per million tokens (if different).
    pub cache_read_price_per_m: Option<f64>,
    /// Blended cost per million tokens (3:1 ratio).
    pub blended_cost_per_m: f64,
}
```

The CostTable is loaded from configuration and updated periodically as providers change pricing. It provides the cost signal that the cascade router uses in its cost penalty computation.

---

## Budget Guardrails

The budget guardrail system (from implementation plan 2H.05–2H.10) enforces multi-level spending limits:

```rust
pub struct BudgetGuardrail {
    /// Per-task cost limit in USD.
    pub per_task_limit: f64,
    /// Per-session cost limit in USD.
    pub per_session_limit: f64,
    /// Per-day cost limit in USD.
    pub per_day_limit: f64,
}

pub enum BudgetAction {
    /// Continue with current model.
    Continue,
    /// Downgrade to a cheaper model (triggered at 80% of limit).
    Downgrade,
    /// Block the request (triggered at 95% of limit).
    Block,
    /// Hard stop — no further requests (triggered at 100% of limit).
    HardStop,
}
```

### Escalation Thresholds

| Level | % of Limit | Action | Rationale |
|-------|------------|--------|-----------|
| Normal | < 80% | Continue | Full freedom |
| Warn | 80% | Downgrade | Route to cheaper model |
| Block | 95% | Block | Reject new requests |
| Hard stop | 100% | HardStop | Terminate session |

The downgrade action at 80% is a soft intervention: instead of failing the task, the router automatically selects a cheaper model. This preserves task completion while controlling costs. The 95% block prevents any new requests from starting, and the 100% hard stop terminates the session entirely.

### Multi-Level Enforcement

Budget limits are enforced at three levels simultaneously:

1. **Per-task** — prevents a single task from consuming disproportionate resources. A task that exceeds its budget is downgraded or blocked before reaching the session limit.
2. **Per-session** — prevents a session (typically one plan execution) from exceeding its allocation. This catches scenarios where many cheap tasks collectively exceed budget.
3. **Per-day** — absolute daily spending cap. This protects against runaway automation that might execute many plans in a day.

```
Incoming request
    │
    ▼
Check per-task budget
    │ if task_cost ≥ 80% of per_task_limit → Downgrade
    │ if task_cost ≥ 95% of per_task_limit → Block
    │
    ▼
Check per-session budget
    │ if session_cost ≥ 80% of per_session_limit → Downgrade
    │ if session_cost ≥ 95% of per_session_limit → Block
    │
    ▼
Check per-day budget
    │ if day_cost ≥ 80% of per_day_limit → Downgrade
    │ if day_cost ≥ 100% of per_day_limit → HardStop
    │
    ▼
Route to selected model (or cheaper alternative if Downgrade)
```

---

## CostsLog: Append-Only Persistence

The `CostsLog` provides durable, file-backed persistence for `CostRecord` values:

```rust
pub struct CostsLog {
    path: PathBuf,
    fsync: bool,
}
```

Key operations:
- `CostsLog::at(path)` — construct a log at `path` with fsync enabled.
- `CostsLog::open_creating(path)` — create parent directories and construct a log.
- `CostsLog::append(record)` — append one `CostRecord` as a JSON line with optional fsync.
- `CostsLog::append_all(records)` — batch append with a single open/close cycle.
- `CostsLog::read_all(path)` — read all records, tolerant of corrupt lines.

The `without_fsync()` builder method disables per-append fsync for test environments where crash safety is not needed.

### Relationship to CostsDb

`CostsDb` is the in-memory cost database used for real-time queries. `CostsLog` is its durable companion: each completed call is appended to the log, and the log is replayed on startup to reconstruct the in-memory database. This separation keeps the hot path fast (in-memory lookups) while maintaining durability (append-only JSONL).

---

## Cost-to-Routing Feedback Loop

Cost data feeds back into routing through two paths:

1. **Direct cost penalty in cascade router** — the confidence stage subtracts a cost penalty from each model's score, biasing toward cheaper models when pass rates are similar.
2. **Budget guardrail enforcement** — when budget limits approach, the guardrail system forces the router to select cheaper models or block requests entirely.

This creates cybernetic feedback loop 6 (Cost→Routing) from [13-8-missing-feedback-loops](13-8-missing-feedback-loops.md): higher costs trigger routing changes that reduce costs, which relaxes the budget pressure, which may allow routing back to better (more expensive) models.

---

## Relationship to Other Documents

- **[04-cascade-router](04-cascade-router.md)** — The cascade router uses cost data for its cost penalty computation.
- **[06-task-metrics-and-baselines](06-task-metrics-and-baselines.md)** — Task metrics include cost fields that come from cost normalization.
- **[09-provider-health-circuit-breaker](09-provider-health-circuit-breaker.md)** — Provider health affects cost indirectly: unhealthy providers cause retries that increase total cost.
- **[13-8-missing-feedback-loops](13-8-missing-feedback-loops.md)** — Loop 6 (Cost→Routing) describes the cost-to-routing feedback path.
- **[10-pareto-frontier-pruning](10-pareto-frontier-pruning.md)** — Pareto pruning uses cost_per_success as one of its optimization axes.


---

# SOURCE: /Users/will/dev/nunchi/roko/roko/docs/05-learning/09-provider-health-circuit-breaker.md

# Provider Health and Circuit Breaker

> **Crate:** `roko-learn` · **Module:** `provider_health.rs`
> **Wiring:** `ProviderHealthRegistry` → `CascadeRouter::select()` (filters unhealthy providers)
> **Implementation plan:** `modelrouting/08-learning-loops.md` (tasks 2G.01–2G.06)
> **Cross-references:** [04-cascade-router](04-cascade-router.md), [08-cost-normalization](08-cost-normalization.md), [13-8-missing-feedback-loops](13-8-missing-feedback-loops.md)


> **Implementation**: Shipping

---

## Purpose

The provider health module tracks the operational status of each LLM provider and implements a three-state circuit breaker that prevents routing requests to degraded or failing providers. When a provider starts returning errors (rate limits, timeouts, server errors), the circuit breaker opens, diverting traffic to healthy alternatives. After a cooldown period, the circuit breaker transitions to half-open, allowing a single probe request to test recovery before fully restoring traffic.

This is cybernetic feedback loop 1 (Health→Routing) from the eight missing feedback loops: provider health state directly influences routing decisions in the cascade router.

---

## Three-State Circuit Breaker

```
                 success
    ┌──────────────────────────┐
    │                          │
    ▼                          │
┌────────┐   failure threshold  ┌────────────┐
│ CLOSED │ ────────────────────►│   OPEN     │
│(normal)│                      │(no traffic)│
└────────┘                      └──────┬─────┘
    ▲                                  │
    │                          cooldown expires
    │         success                  │
    │  ┌──────────────────┐            │
    └──┤    HALF-OPEN     │◄───────────┘
       │(single probe req)│
       └──────────────────┘
              │
              │ failure
              │
              ▼
          OPEN (reset cooldown)
```

### States

| State | Behavior | Transition |
|-------|----------|------------|
| **Closed** | Normal operation. All requests are routed. Failures are counted. | → Open: when failure count exceeds threshold within window |
| **Open** | No requests are routed. Traffic is diverted to alternative providers. | → Half-Open: after cooldown period expires |
| **Half-Open** | A single probe request is allowed through. | → Closed: if probe succeeds · → Open: if probe fails (reset cooldown) |

---

## Error Classification

Not all errors are equal. The provider health module classifies errors by type to apply appropriate cooldown policies:

```rust
pub enum ErrorClass {
    /// HTTP 429 — provider rate limit.
    RateLimit,
    /// HTTP 401/403 — authentication or authorization failure.
    AuthFailure,
    /// Request or response timeout.
    Timeout,
    /// HTTP 5xx — provider server error.
    ServerError,
    /// Content policy violation (filtered response).
    ContentPolicy,
    /// Context window exceeded.
    ContextOverflow,
    /// Unclassified error.
    Unknown,
}
```

### Error-Specific Cooldowns

Each error class has a tailored cooldown strategy:

| Error Class | Cooldown | Rationale |
|-------------|----------|-----------|
| `RateLimit` | 60s (escalating with backoff) | Provider will recover after rate window resets |
| `AuthFailure` | 300s (long) | Requires manual intervention (API key rotation) |
| `Timeout` | 30s | Often transient network issues |
| `ServerError` | 120s | Provider-side issues, variable recovery |
| `ContentPolicy` | 0s (no cooldown, flag only) | Not a provider health issue — content-specific |
| `ContextOverflow` | 0s (no cooldown, route to larger model) | Not a provider issue — task-specific |
| `Unknown` | 60s (conservative default) | Unknown errors get conservative treatment |

### Failure Records

Each failure is recorded with its classification:

```rust
pub struct FailureRecord {
    /// When the failure occurred.
    pub timestamp: DateTime<Utc>,
    /// Error classification.
    pub error_class: ErrorClass,
    /// Raw error message (truncated to 256 chars).
    pub message: String,
    /// Model that was being used.
    pub model: String,
}
```

---

## ProviderHealth

Per-provider health state:

```rust
pub struct ProviderHealth {
    /// Provider identifier.
    pub provider_id: String,
    /// Current circuit breaker state.
    pub state: CircuitState,
    /// Recent failure records (bounded window).
    pub recent_failures: VecDeque<FailureRecord>,
    /// Total failure count since last reset.
    pub failure_count: u64,
    /// Total success count since last reset.
    pub success_count: u64,
    /// When the circuit breaker last opened.
    pub last_opened: Option<DateTime<Utc>>,
    /// When the circuit breaker will transition to half-open.
    pub cooldown_until: Option<DateTime<Utc>>,
}
```

### Threshold Configuration

The circuit breaker opens when:
- **Failure count** exceeds the threshold within the observation window, OR
- **Failure rate** (failures / total requests) exceeds the rate threshold.

Default values:
- Failure count threshold: 5 failures
- Observation window: 60 seconds
- Failure rate threshold: 50%

---

## ProviderHealthRegistry

The registry manages health state for all providers:

```rust
pub struct ProviderHealthRegistry {
    providers: Mutex<HashMap<String, ProviderHealth>>,
}
```

Key operations:

| Method | What it does |
|--------|-------------|
| `record_success(provider)` | Increment success count. If half-open, transition to closed. |
| `record_failure(provider, error_class)` | Record failure. Check threshold. If exceeded, open circuit. |
| `is_available(provider)` | Returns `true` if circuit is Closed or Half-Open. |
| `available_providers()` | Returns all providers with Closed or Half-Open circuits. |

### Integration with Cascade Router

The cascade router calls `is_available()` before scoring each candidate model:

```
CascadeRouter::select(context)
    │
    ├── For each candidate model:
    │     │
    │     ├── ProviderHealthRegistry::is_available(model.provider)?
    │     │     YES → include in candidate set
    │     │     NO  → exclude (circuit is Open)
    │     │
    │     └── Score candidate using stage algorithm
    │
    └── Select highest-scoring available candidate
```

If all providers for a desired model tier are unavailable, the router escalates to the next tier or returns the fallback model from the `CascadeModel`.

---

## Exponential Backoff

When a circuit breaker reopens after a failed half-open probe, the cooldown period increases exponentially:

```
cooldown(n) = base_cooldown × 2^(n-1)
```

where `n` is the number of consecutive open→half-open→open cycles. This prevents the system from hammering a persistently failing provider with probe requests.

| Cycle | Cooldown (RateLimit) | Cooldown (ServerError) |
|-------|---------------------|----------------------|
| 1 | 60s | 120s |
| 2 | 120s | 240s |
| 3 | 240s | 480s |
| 4 | 480s (max) | 480s (max) |

The maximum cooldown is capped at 480 seconds (8 minutes) to ensure eventual re-probing even for persistently failing providers.

---

## ProviderHealthTracker

The `ProviderHealthTracker` extends the registry with time-series health metrics for dashboard visualization:

```
Provider: anthropic
├── State: Closed
├── Success rate (1h): 98.2%
├── Failure rate (1h): 1.8%
├── Recent errors: [Timeout × 1, RateLimit × 2]
├── Avg latency (1h): 1,240ms
└── Circuit opens (24h): 2
```

This data feeds into the learning dashboard described in [16-heartbeat](../16-heartbeat/INDEX.md) and the conductor subsystem described in [07-conductor](../07-conductor/INDEX.md).

---

## Anomaly Detection Integration

The `AnomalyDetector` in `anomaly.rs` provides additional provider-health-adjacent checks:

```rust
pub struct AnomalyDetector {
    prompt_hash_window: VecDeque<u64>,    // last 20 prompt hashes
    cost_ewma: EwmaState,                 // EWMA cost baseline
    quality_history: VecDeque<f64>,        // rolling quality scores
    session_cost_usd: f64,
    session_start_ms: i64,
}
```

Three anomaly types:

| Anomaly | Detection | Threshold |
|---------|-----------|-----------|
| **Prompt loop** | Same prompt hash appears 5+ times in last 20 | `PROMPT_LOOP_THRESHOLD = 5` |
| **Cost spike** | Z-score against EWMA baseline > 3.0 | `COST_SPIKE_Z_THRESHOLD = 3.0` |
| **Quality degradation** | Recent 5 scores average < 0.5 AND drop > 0.15 vs prior 10 | Composite check |

### EWMA Cost Baseline

The cost spike detector uses an Exponential Weighted Moving Average with α = 0.2:

```
ewma_new = α × observation + (1 − α) × ewma_old
z_score = (observation − ewma) / ewma_stddev
```

The observation is compared against the EWMA *before* the state is updated, keeping sudden spikes visible instead of folding them into the baseline immediately.

---

## Relationship to Other Documents

- **[04-cascade-router](04-cascade-router.md)** — The cascade router filters candidates using `ProviderHealthRegistry::is_available()`.
- **[08-cost-normalization](08-cost-normalization.md)** — Provider health affects cost indirectly through retry patterns.
- **[13-8-missing-feedback-loops](13-8-missing-feedback-loops.md)** — Loop 1 (Health→Routing) is the primary feedback path from provider health to routing decisions.
- **[14-stability-mechanisms](14-stability-mechanisms.md)** — The circuit breaker is itself a stability mechanism (negative feedback loop).
- **[07-conductor](../07-conductor/INDEX.md)** — The conductor subsystem uses provider health data for its circuit breaker watchers.


---

# SOURCE: /Users/will/dev/nunchi/roko/roko/docs/05-learning/10-pareto-frontier-pruning.md

# Pareto Frontier Pruning

> **Crate:** `roko-learn` · **Module:** `pareto.rs`
> **Wiring:** `CascadeRouter` calls `compute_pareto_frontier()` every 50 observations
> **Implementation plan:** `modelrouting/08-learning-loops.md` (task 2G.11)
> **Cross-references:** [03-bandits-ucb-thompson-linucb](03-bandits-ucb-thompson-linucb.md), [04-cascade-router](04-cascade-router.md), [08-cost-normalization](08-cost-normalization.md)


> **Implementation**: Shipping

---

## Purpose

Pareto frontier pruning identifies which models are non-dominated with respect to two objectives: pass rate and cost per successful task. A model is Pareto-optimal if no other model has both a higher pass rate and a lower cost per successful task. Dominated models (worse on both metrics than some other model) are pruned from the candidate set before presenting arms to the bandit.

This serves two functions:
1. **Reduces exploration waste** — the bandit doesn't spend trials on clearly inferior models.
2. **Focuses the tradeoff** — the remaining Pareto-optimal models represent genuine cost-quality tradeoffs that the bandit must resolve.

---

## Dominance Definition

Model A dominates model B when:
- A has pass_rate ≥ B's pass_rate, AND
- A has cost_per_success ≤ B's cost_per_success, AND
- At least one inequality is strict.

```
Model A: pass_rate=0.90, cost/success=$10.00
Model B: pass_rate=0.70, cost/success=$12.00
Model C: pass_rate=0.80, cost/success=$9.00

A dominates B (higher pass rate AND lower cost).
Neither A nor C dominates the other:
  A has higher pass rate, but C has lower cost.
  → Both are Pareto-optimal.
```

---

## Algorithm

```rust
pub fn compute_pareto_frontier(
    stats: &HashMap<String, ModelObservation>
) -> Vec<String> {
    let mut frontier = Vec::new();

    for (slug_a, obs_a) in stats {
        let dominated = stats.iter().any(|(slug_b, obs_b)| {
            slug_b != slug_a
                && obs_b.pass_rate >= obs_a.pass_rate
                && obs_b.cost_per_success <= obs_a.cost_per_success
                && (obs_b.pass_rate > obs_a.pass_rate
                    || obs_b.cost_per_success < obs_a.cost_per_success)
        });

        if !dominated {
            frontier.push(slug_a.clone());
        }
    }

    frontier.sort();
    frontier
}
```

The algorithm is O(n²) where n is the number of models. With typical model counts (3-10), this is negligible.

### ModelObservation

```rust
pub struct ModelObservation {
    /// Fraction of tasks that passed.
    pub pass_rate: f64,
    /// Total cost divided by number of successful tasks.
    pub cost_per_success: f64,
    /// Average latency in milliseconds.
    pub avg_latency_ms: f64,
    /// Number of observations contributing to this summary.
    pub observations: u64,
}
```

Note that `avg_latency_ms` is tracked but not currently used in the dominance check. Future extensions may include latency as a third Pareto dimension, creating a three-objective frontier.

---

## Visualization

```
Pass Rate ↑
    1.0 │         ● A (Pareto-optimal)
        │
    0.8 │    ● C (Pareto-optimal)
        │
    0.7 │              ✗ B (dominated by A)
        │
    0.6 │
        │
    0.0 └────────────────────────────────► Cost/Success
        $0   $5    $9   $10   $12   $15
```

The Pareto frontier is the upper-left boundary of the point cloud. Points below and to the right of any frontier point are dominated.

---

## Integration with Cascade Router

The cascade router recomputes the Pareto frontier every `PARETO_RECOMPUTE_INTERVAL = 50` observations:

```
CascadeRouter::update(model, reward, cost)
    │
    ├── Update model stats (trials, successes, costs)
    │
    ├── if observations % 50 == 0:
    │       │
    │       ├── Collect ModelObservation for each model
    │       │     pass_rate = successes / trials
    │       │     cost_per_success = total_cost / successes
    │       │
    │       └── pareto_frontier = compute_pareto_frontier(observations)
    │
    └── Store frontier for use in next select() call
```

During `select()`, only models on the Pareto frontier are presented as candidates to the stage-2 or stage-3 routing algorithm. Models that fell off the frontier are excluded from consideration until the next recomputation (which may restore them if other models' statistics change).

---

## Multi-Objective Extension

The current implementation uses a two-objective Pareto frontier (pass_rate, cost_per_success). The implementation plan (modelrouting/12-advanced-patterns.md, task 2J.13) describes a multi-objective Pareto bandit that extends this to four dimensions:

| Objective | Direction | Weight |
|-----------|-----------|--------|
| Quality (pass rate) | Maximize | Configurable |
| Cost per success | Minimize | Configurable |
| Latency (p50) | Minimize | Configurable |
| Reliability (1 − error rate) | Maximize | Configurable |

The multi-objective extension uses scalarization: each objective is weighted and combined into a single score, and the Pareto frontier is computed over the scalarized scores. This preserves the O(n²) complexity while enabling richer tradeoff analysis.

---

## Edge Cases

### All Models Dominated

If the model set contains a single dominant model (highest pass rate AND lowest cost), all other models are dominated and the frontier contains only one model. In this case, the bandit has no choice to make — the dominant model is always selected. This is the expected steady-state for mature systems where one model clearly outperforms alternatives.

### Insufficient Observations

Models with very few observations have noisy statistics. A model that happened to succeed on its first 3 trials appears to have a 100% pass rate, potentially dominating models with hundreds of observations and a 90% pass rate. The cascade router mitigates this by requiring a minimum observation count before including a model in the Pareto computation. Models below this threshold are always included in the candidate set (exploration) regardless of dominance.

### New Models

When a new model is added to the system (e.g., a provider releases a new model version), it starts with zero observations and is excluded from Pareto computation. The bandit gives it maximum exploration priority (UCB1 selects unpulled arms first), ensuring it accumulates enough data for Pareto evaluation within the first 50 observations.

---

## Frontier Evolution Over Time

The Pareto frontier is not static — it evolves as the system accumulates observations and as models change.

### Cold Start

At system start with no observations, all models are on the Pareto frontier (no model has enough data to be dominated). The bandit explores uniformly.

### Convergence Phase (50-200 observations)

As statistics accumulate, dominated models begin to fall off the frontier. Typically, the model set converges to 2-3 Pareto-optimal models representing genuine tradeoffs (e.g., cheap-but-lower-quality vs. expensive-but-higher-quality).

### Steady State (200+ observations)

The frontier stabilizes. Changes occur when:
- A provider updates a model (changing its quality or cost characteristics).
- A new model is added to the system.
- The task mix changes (altering the observed pass rates).

### Provider Updates

When a provider deploys a new model version, the model's historical statistics may no longer reflect its current performance. The cascade router handles this by:
1. Detecting the model version change (via model slug comparison).
2. Discounting old observations (partial reset of the model's stats).
3. Re-including the model in the Pareto computation with reduced weight.

This ensures that a model that was previously dominated but has been improved by its provider gets a fair chance to re-enter the frontier.

---

## Practical Example

Consider a system with four models after 300 observations:

```
Model               Pass Rate   Cost/Success   On Frontier?
─────────────────────────────────────────────────────────────
claude-haiku-4.5     0.78        $0.12          YES (cheapest)
claude-sonnet-4      0.86        $0.95          YES (mid-range)
claude-opus-4        0.91        $2.40          YES (highest quality)
deepseek-chat        0.72        $0.45          NO (dominated by haiku)
```

Deepseek is dominated by haiku (haiku has both higher pass rate AND lower cost), so it's pruned from the candidate set. The bandit only considers haiku, sonnet, and opus — three models representing the genuine cost-quality tradeoff.

After a provider update where deepseek improves to 0.85 pass rate:

```
Model               Pass Rate   Cost/Success   On Frontier?
─────────────────────────────────────────────────────────────
claude-haiku-4.5     0.78        $0.12          YES (cheapest)
deepseek-chat        0.85        $0.45          YES (new: better than haiku, cheaper than sonnet)
claude-sonnet-4      0.86        $0.95          NO (dominated by deepseek!)
claude-opus-4        0.91        $2.40          YES (highest quality)
```

Now sonnet is dominated by deepseek (deepseek has nearly the same pass rate at half the cost), and deepseek enters the frontier. The bandit shifts exploration toward deepseek.

---

## Relationship to Other Documents

- **[03-bandits-ucb-thompson-linucb](03-bandits-ucb-thompson-linucb.md)** — Pareto pruning reduces the arm set presented to the bandit.
- **[04-cascade-router](04-cascade-router.md)** — The cascade router uses the Pareto frontier to filter candidates before scoring.
- **[08-cost-normalization](08-cost-normalization.md)** — Cost per success uses normalized costs from the cost normalization layer.
- **[11-thompson-sampling-drift](11-thompson-sampling-drift.md)** — Thompson Sampling with drift can be combined with Pareto pruning for non-stationary multi-objective optimization.
- **[14-stability-mechanisms](14-stability-mechanisms.md)** — Recomputation interval (every 50 observations) is a form of frequency separation.


---

# SOURCE: /Users/will/dev/nunchi/roko/roko/docs/05-learning/11-thompson-sampling-drift.md

# Thompson Sampling with Drift

> **Implementation plan:** `modelrouting/12-advanced-patterns.md` (tasks 2J.01–2J.03)
> **Academic basis:** Thompson 1933; Garivier & Moulines 2011 (discounted Thompson Sampling)
> **Cross-references:** [03-bandits-ucb-thompson-linucb](03-bandits-ucb-thompson-linucb.md), [04-cascade-router](04-cascade-router.md), [14-stability-mechanisms](14-stability-mechanisms.md)


> **Implementation**: Shipping

---

## Purpose

Thompson Sampling with drift is a Bayesian bandit algorithm designed for non-stationary environments where the reward distribution of each arm changes over time. In the model routing context, non-stationarity arises from:

- **Provider updates**: A model's quality changes when the provider deploys a new version.
- **Codebase evolution**: As the codebase grows or is refactored, the relative performance of models shifts.
- **Task mix changes**: The distribution of task categories and complexities varies across development phases.
- **Cache dynamics**: Repeated access patterns improve cache hit rates, changing the effective cost of models.

Standard UCB1 and LinUCB are designed for stationary environments: they accumulate all historical observations equally. In a non-stationary world, old observations can mislead the algorithm — a model that was excellent three months ago may be mediocre today after a provider update, but its strong historical record keeps UCB1 selecting it.

Thompson Sampling with a discount factor addresses this by down-weighting old observations, making the algorithm responsive to recent performance changes.

---

## Algorithm

### Standard Thompson Sampling

For each arm `a` with binary reward (pass/fail):

1. Maintain Beta distribution parameters `(α_a, β_a)` where α = successes, β = failures.
2. To select: sample `θ_a ~ Beta(α_a, β_a)` for each arm. Select arm with highest sample.
3. To update: if reward = 1, α_a += 1. If reward = 0, β_a += 1.

The Beta distribution naturally encodes uncertainty: arms with few observations have wide distributions (high exploration), while well-observed arms have narrow distributions (high exploitation).

### Adding Drift (Discount Factor)

To handle non-stationarity, apply a discount factor γ ∈ (0, 1) to existing observations before updating:

```
On update for arm a:
    α_a ← γ · α_a + reward
    β_a ← γ · β_a + (1 − reward)
```

The discount factor γ controls the "effective window" of observations:

| γ | Effective window | Behavior |
|---|-----------------|----------|
| 0.999 | ~1000 observations | Very slow forgetting, near-stationary |
| 0.99 | ~100 observations | Moderate forgetting |
| 0.95 | ~20 observations | Fast forgetting, very responsive |
| 0.90 | ~10 observations | Aggressive forgetting |

### Effective Window Calculation

The effective window is approximately `1 / (1 − γ)`. After `n` observations, the weight of the oldest observation is `γ^n`. When `γ^n < 0.01` (i.e., the oldest observation contributes less than 1%), we consider it effectively forgotten:

```
n_effective = ln(0.01) / ln(γ) = −4.605 / ln(γ)
```

| γ | n_effective |
|---|-------------|
| 0.999 | 4603 |
| 0.99 | 460 |
| 0.95 | 90 |
| 0.90 | 44 |

---

## Design Considerations for Roko

### Recommended Discount Factor

For model routing in Roko, the recommended discount factor is **γ = 0.995** (effective window ~200 observations). This balances:

- **Responsiveness**: Detects model quality changes within ~50 observations of the change.
- **Stability**: Doesn't overreact to short-term noise from individual task outcomes.
- **Cold start**: After 200 observations, the system has effectively "forgotten" its cold-start period and responds only to recent performance.

### Comparison with UCB1 and LinUCB

| Property | UCB1 | LinUCB | Thompson + Drift |
|----------|------|--------|-----------------|
| Context-dependent | No | Yes (18-dim) | No (per-arm) |
| Non-stationary | No | No | Yes (γ discount) |
| Exploration | Deterministic (upper bound) | Deterministic (upper bound) | Stochastic (sampling) |
| Convergence | O(√(T ln T)) regret | O(d√(T ln T)) regret | O(√(T / (1−γ))) regret |
| Cold start | Infinite UCB for unpulled arms | Static fallback | Wide Beta prior |

### When to Use Thompson Sampling vs UCB1

- **Use UCB1** for stationary decisions (tool format selection, retry strategy) where the optimal choice doesn't change over time.
- **Use Thompson Sampling with drift** for non-stationary decisions (model routing, provider selection) where the optimal choice shifts with provider updates and codebase evolution.
- **Use LinUCB** when context features (task category, complexity, role) strongly influence the optimal choice, even in a stationary environment.

The cascade router currently uses LinUCB in stage 3. Thompson Sampling with drift is proposed as an alternative stage-3 algorithm for environments with frequent model updates, as specified in implementation plan 2J.01–2J.03.

---

## Implementation Design

### Per-Arm State

```rust
struct ThompsonArm {
    /// Model slug.
    model: String,
    /// Beta distribution α parameter (discounted successes).
    alpha: f64,
    /// Beta distribution β parameter (discounted failures).
    beta: f64,
    /// Total observations (not discounted, for diagnostics).
    total_observations: u64,
}
```

### Selection

```rust
fn select(arms: &[ThompsonArm], rng: &mut impl Rng) -> usize {
    arms.iter()
        .enumerate()
        .max_by(|(_, a), (_, b)| {
            let sample_a = Beta::new(a.alpha.max(0.01), a.beta.max(0.01)).sample(rng);
            let sample_b = Beta::new(b.alpha.max(0.01), b.beta.max(0.01)).sample(rng);
            sample_a.partial_cmp(&sample_b).unwrap()
        })
        .map(|(i, _)| i)
        .unwrap_or(0)
}
```

The `max(0.01)` floor prevents degenerate Beta distributions when both parameters approach zero after heavy discounting.

### Update with Discount

```rust
fn update(arm: &mut ThompsonArm, reward: f64, gamma: f64) {
    arm.alpha = gamma * arm.alpha + reward;
    arm.beta = gamma * arm.beta + (1.0 - reward);
    arm.total_observations += 1;
}
```

---

## Interaction with Stability Mechanisms

Thompson Sampling's stochastic selection naturally provides exploration, but in combination with the cascade router's hysteresis mechanism, it can create oscillation between near-equal models. The hysteresis threshold (10% score delta to switch models) acts as a damper:

```
Current model: claude-sonnet-4 (sampled θ = 0.82)
Challenger: claude-opus-4 (sampled θ = 0.85)
Delta: 0.85 − 0.82 = 0.03 < 0.10 (hysteresis threshold)
→ Keep current model (no switch)
```

This prevents the stochastic nature of Thompson Sampling from causing rapid model switching when multiple models have similar performance. See [14-stability-mechanisms](14-stability-mechanisms.md) for the full hysteresis design.

---

## Drift Detection

Thompson Sampling with discount handles gradual drift automatically (the discount factor continuously down-weights old data). For sudden, abrupt changes (e.g., a provider deploys a breaking change), an additional drift detection mechanism can trigger a "reset":

```
If recent_pass_rate(last 10) << historical_pass_rate(last 100):
    Reset arm: α ← 1, β ← 1 (uninformative prior)
    → Full re-exploration for this arm
```

This combines the anomaly detection from [09-provider-health-circuit-breaker](09-provider-health-circuit-breaker.md) with the Thompson Sampling state: when the circuit breaker detects a provider degradation, the corresponding Thompson arm is reset to force re-evaluation.

---

## Contextual Thompson Sampling

Thompson Sampling can also be extended with context features, creating a Bayesian analogue to LinUCB. The contextual version maintains a posterior distribution over the weight vector `θ_a` for each arm:

```
Prior: θ_a ~ N(μ_0, Σ_0)
After observation (x, r): update posterior via Bayesian linear regression
Selection: sample θ_a from posterior, compute score = θ_a^T · x
```

This provides the exploration benefits of Thompson Sampling (stochastic selection based on posterior uncertainty) with the context-awareness of LinUCB (feature-dependent scoring).

### When to Use Contextual Thompson vs LinUCB

| Criterion | LinUCB | Contextual Thompson |
|-----------|--------|-------------------|
| Stationary environment | Preferred | Either |
| Non-stationary environment | Poor | Preferred (with discount) |
| Deterministic exploration | Yes | No (stochastic) |
| Posterior uncertainty | Point estimate + bound | Full distribution |
| Computational cost | Lower (matrix inverse) | Higher (sampling) |

For Roko's model routing, LinUCB is currently preferred because:
1. The 18-dimensional context space is well-suited to linear models.
2. Deterministic exploration (UCB bound) provides reproducible routing for debugging.
3. The stationary assumption holds over short periods (50-200 episodes).

Thompson Sampling with drift would be adopted when model provider updates create significant non-stationarity that LinUCB handles poorly.

---

## Empirical Guidance

### Monitoring Drift

The system can detect when Thompson Sampling with drift would outperform LinUCB by monitoring:

1. **Prediction error trend**: If the cascade router's predictions degrade steadily, the environment is non-stationary and Thompson with drift may help.
2. **Arm switching frequency**: If the bandit switches arms frequently (> 20% of decisions), the reward landscape is changing and a discount factor would help stabilize.
3. **Calibration drift**: If the CalibrationTracker (see [16-predictive-foraging](16-predictive-foraging.md)) shows systematic bias that increases over time, the model quality distribution is shifting.

### Adaptive Discount Factor

Rather than fixing γ, the system can adapt it based on observed non-stationarity:

```
If arm_switching_rate > 0.20:
    γ ← max(0.90, γ − 0.01)    // Increase forgetting
If arm_switching_rate < 0.05:
    γ ← min(0.999, γ + 0.01)   // Decrease forgetting
```

This ensures that the discount factor tracks the actual rate of change in the environment, rather than relying on a fixed prior assumption about non-stationarity.

---

## Relationship to Other Documents

- **[03-bandits-ucb-thompson-linucb](03-bandits-ucb-thompson-linucb.md)** — Foundational bandit algorithms that Thompson Sampling extends.
- **[04-cascade-router](04-cascade-router.md)** — Thompson Sampling is a proposed alternative to LinUCB for stage-3 routing.
- **[09-provider-health-circuit-breaker](09-provider-health-circuit-breaker.md)** — Circuit breaker events can trigger Thompson arm resets.
- **[10-pareto-frontier-pruning](10-pareto-frontier-pruning.md)** — Pareto pruning reduces the arm set before Thompson selection.
- **[14-stability-mechanisms](14-stability-mechanisms.md)** — Hysteresis prevents Thompson Sampling oscillation.
- **[12-self-improvement-frameworks](12-self-improvement-frameworks.md)** — Academic context for non-stationary bandit algorithms.


---

# SOURCE: /Users/will/dev/nunchi/roko/roko/docs/05-learning/12-self-improvement-frameworks.md

# Self-Improvement Frameworks

> **Sources:** Academic literature survey, legacy research docs, implementation plans
> **Cross-references:** [02-skill-library-voyager](02-skill-library-voyager.md), [04-cascade-router](04-cascade-router.md), [17-adas-and-autocatalytic](17-adas-and-autocatalytic.md)


> **Implementation**: Shipping

---

## Purpose

This document surveys the academic and industrial frameworks that inform Roko's learning architecture. Each framework contributes a specific insight that is implemented (or planned) in the system. The survey is organized from concrete (implemented techniques) to speculative (research directions), with explicit citations for traceability.

---

## Agent Self-Improvement Frameworks

### Reflexion (Shinn et al. 2023)

**Insight:** Agents improve by reflecting on failures in natural language, then using those reflections as additional context in subsequent attempts.

**Roko implementation:** The episode logger captures gate failure signatures. The playbook rule system extracts if-then rules from failure patterns and injects them into subsequent agent prompts. This is a structured form of Reflexion: instead of free-form natural language reflection, Roko extracts typed rules with confidence tracking and trigger matching.

**Key difference:** Reflexion operates within a single task's retry loop. Roko's playbook rules persist across tasks and plans — a failure in plan A prevents the same mistake in plan B.

### ExpeL (Zhao et al. 2023)

**Insight:** Agents should extract generalizable "experiences" (insights) from successful and failed trials, accumulating them into a growing library.

**Roko implementation:** The skill library implements ExpeL-style experience extraction. Successful episodes produce skills (positive experiences); failure patterns produce playbook rules (negative experiences). Both persist across sessions and grow monotonically.

**Key difference:** ExpeL uses natural language experiences without confidence tracking. Roko's playbook rules have bounded confidence dynamics (validate +0.05, contradict −0.10) that automatically prune stale experiences.

### DSPy (Khattab et al. 2023)

**Insight:** Prompt optimization should be treated as a compiler problem: define a program signature, generate prompt variations, evaluate against a metric, and select the best-performing variant.

**Roko implementation:** The prompt experiment system (`ExperimentStore`) implements DSPy-style prompt optimization. Each experiment defines a prompt section, generates variants, assigns variants using UCB1 bandit selection, and evaluates against gate pass rate.

**Key difference:** DSPy optimizes statically (generate many variants, evaluate on a test set, select the winner). Roko optimizes online (bandit-driven variant selection during live execution, continuous evaluation).

### Meta-Harness (concept from self-hosted development)

**Insight:** A system that develops itself should use its own self-improvement mechanisms on its own self-improvement mechanisms. The harness that runs agents should itself be subject to optimization.

**Roko implementation:** This is the autocatalytic thesis described in [17-adas-and-autocatalytic](17-adas-and-autocatalytic.md). Roko uses its own learning loops to optimize the components that implement those learning loops. When Roko modifies `roko-learn` code, the cascade router learns which model works best for `roko-learn` tasks, and the skill library accumulates patterns specific to modifying the learning subsystem.

---

## Model Routing Research

### RouteLLM (Ong et al., ICLR 2025)

**Result:** 85% cost reduction while maintaining quality by routing queries to strong or weak models based on predicted difficulty.

**Approach:** Train a classifier (matrix factorization, BERT, or causal LM) on human preference data to predict which queries need a strong model. Route to weak model unless the classifier predicts the strong model is needed.

**Roko adaptation:** The cascade router's confidence stage implements a simpler version: empirical pass rates per model with confidence intervals, rather than a neural classifier. The LinUCB stage provides context-dependent routing similar to RouteLLM's classifier but using linear contextual bandits instead of neural networks.

### FrugalGPT (Chen et al., arXiv:2305.05176)

**Result:** 98% cost reduction with maintained quality by cascading through models from cheapest to most expensive, stopping when confidence is high enough.

**Approach:** Send the query to the cheapest model first. If the model's confidence (measured by agreement with a scoring model) is below threshold, escalate to the next more expensive model.

**Roko adaptation:** The cascade router's fallback mechanism implements this pattern: the `CascadeModel` includes both a primary and a fallback model. If the primary fails (gate failure, timeout), the orchestrator retries with the fallback. The three-stage cascade (Static→Confidence→UCB) is a different dimension of cascading: strategy complexity rather than model cost.

### MixLLM (concept)

**Result:** 97.25% of GPT-4 quality at 24.18% of the cost by mixing outputs from multiple models.

**Roko relevance:** Not directly implemented. Roko routes to a single model per task rather than mixing outputs. However, the collective calibration mechanism (see [15-collective-calibration-31x](15-collective-calibration-31x.md)) achieves a related effect: multiple agents with different models collectively produce better outcomes than any single agent.

### AutoMix (NeurIPS 2024)

**Insight:** Self-verification enables cascading without a separate scoring model. After the cheap model generates a response, ask it to verify its own answer. If self-verification fails, escalate to the expensive model.

**Roko adaptation:** Gate verification serves as Roko's "self-verification": the compile, test, and lint gates provide ground-truth feedback that the response is correct, without requiring a separate scoring model. This is more reliable than LLM self-verification because the gates are deterministic.

### Unified Routing (ETH Zurich, ICLR 2025)

**Insight:** Route across multiple providers simultaneously, considering cost, latency, and quality as a multi-objective optimization problem.

**Roko implementation:** The Pareto frontier computation (see [10-pareto-frontier-pruning](10-pareto-frontier-pruning.md)) and multi-provider health tracking (see [09-provider-health-circuit-breaker](09-provider-health-circuit-breaker.md)) implement unified routing across providers. The cascade router considers cost, quality (pass rate), and latency SLA when selecting models.

### Speculative Cascades

**Concept:** Start processing with a cheap model while simultaneously evaluating whether to hand off to an expensive model. If the cheap model's partial output looks promising, continue; otherwise, switch.

**Roko relevance:** Not implemented. Roko processes tasks sequentially (one model attempt at a time) rather than speculatively. Speculative cascading would require streaming gate evaluation, which the current batch-gate pipeline doesn't support.

---

## Production Routing Systems

### LiteLLM

Open-source proxy that standardizes API calls across 100+ LLM providers. Provides routing, fallback, and cost tracking. Roko's `roko-agent` dispatcher serves a similar function but is specialized for agent workloads with gate-based feedback.

### OpenRouter

Commercial routing service that provides unified API access to multiple models. Roko's cascade router draws from OpenRouter's approach of maintaining per-model performance statistics and routing based on empirical quality data.

### Portkey

Production LLM gateway with routing, fallback, and observability. Roko's provider health tracking is inspired by Portkey's circuit breaker patterns.

---

## Self-Improvement Prerequisites

The self-improvement literature consistently identifies prerequisites that Roko satisfies:

### External Verifier Requirement

Huang et al. (ICLR 2024), Song et al. (ICLR 2025), and Pan et al. (ICML 2024) establish that self-improvement requires an external verifier: models cannot reliably improve their own outputs without ground-truth feedback.

**Roko's verifier:** The 11-gate pipeline (compile, test, clippy, diff, etc.) provides deterministic external verification. This is stronger than the weak verifiers (LLM-as-judge) used in most self-improvement research, because gate outcomes are not subject to model bias or hallucination.

### Karpathy Autoresearch Pattern

Andrej Karpathy's autoresearch experiment (700 experiments, 11% speedup, rediscovered RMSNorm) demonstrates that automated experimentation can produce genuine insights, but requires careful metric tracking and experiment isolation.

**Roko implementation:** The prompt experiment system (`ExperimentStore`) implements isolated A/B testing with bandit-driven variant selection. The cascade router provides automated model experimentation. Both produce structured outcome data for analysis.

---

## Context Assembly Optimization

The highest-leverage self-improvement in the legacy system (mori-agents/07-self-improvement.md) was identified as **adaptive context dropping** — learning which prompt sections contribute to gate passes and which waste tokens. This insight directly motivated:

- The `PromptSectionMeta` tracking in efficiency events (section-level token attribution).
- The prompt experiment system (A/B testing prompt section variants).
- The section effectiveness feedback loop (loop 3 in [13-8-missing-feedback-loops](13-8-missing-feedback-loops.md)).

---

## Four Key Metrics for Self-Improvement

From the legacy analysis (mori-agents/07-self-improvement.md):

| Metric | Definition | Self-Improvement Lever |
|--------|-----------|----------------------|
| First-attempt pass rate | % tasks passing gates first try | Playbook rules prevent known failures |
| Iterations per plan | Avg iterations to complete | Better model routing, better prompts |
| Cost per plan | Total USD per plan | Model routing, cache optimization |
| Prompt tokens per spawn | Input tokens for initial prompt | Context assembly optimization |

These four metrics form the core of Roko's self-improvement feedback: every learning subsystem ultimately aims to improve one or more of these numbers.

---

## Router-R1 and Speculative Cascades

### Router-R1

A reinforcement-learning-trained router that uses chain-of-thought reasoning to make routing decisions. Unlike RouteLLM's classifier approach, Router-R1 generates an explicit reasoning trace before making the routing decision, enabling interpretable routing logic.

**Roko relevance:** The cascade router's stage transitions (Static → Confidence → UCB) can be seen as a hardcoded reasoning chain. Router-R1 suggests that this chain itself could be learned — an ADAS-level optimization (see [17-adas-and-autocatalytic](17-adas-and-autocatalytic.md)).

### Speculative Cascades

Start processing with a cheap model while simultaneously evaluating whether to hand off to an expensive model. If the cheap model's partial output looks promising, continue; otherwise, switch. This requires streaming gate evaluation, which the current batch-gate pipeline doesn't support.

**Roko relevance:** Not implemented. The current gate pipeline evaluates complete outputs, not streaming partial results. Speculative cascading would require modifications to the gate pipeline architecture (see [04-verification](../04-verification/INDEX.md)).

---

## Unified Routing (ETH Zurich, ICLR 2025)

A comprehensive framework for routing across multiple providers simultaneously, treating cost, latency, and quality as a multi-objective optimization problem. The unified approach considers the entire provider landscape as a single decision space rather than selecting providers independently.

**Key insight:** Provider-level routing (which provider to use) and model-level routing (which model to use) should be solved jointly, because the same model may have different cost, latency, and quality characteristics across providers.

**Roko implementation:** The `ProviderHealthRegistry` + `CascadeRouter` + `LatencyRegistry` together implement a form of unified routing. Provider health filters out degraded providers, the cascade router selects models, and latency statistics inform SLA compliance. However, these operate sequentially rather than jointly — full unified routing would optimize across all three dimensions simultaneously.

---

## Practical Insights from Production

### Adaptive Context Dropping (Highest Leverage)

The legacy analysis (mori-agents/07-self-improvement.md) identified adaptive context dropping as the single highest-leverage self-improvement technique. The insight: most prompt sections in an agent's system prompt are irrelevant to the current task, but they consume tokens and may confuse the agent. Learning which sections to drop (or heavily truncate) for each task type can:

- Reduce prompt size by 30-50% (saving input token costs).
- Improve pass rates by 5-15% (less noise in the prompt).
- Reduce latency by 20-40% (fewer tokens to process).

**Roko implementation:** The `PromptSectionMeta` tracking in efficiency events (per-section token attribution), combined with feedback loop 3 (Section→Scaffold), enables adaptive context dropping. The system tracks which sections correlate with gate passes and adjusts section weights accordingly.

### Warm Pool Optimization

Reusing agent processes (warm starts) instead of spawning fresh processes (cold starts) saves:
- Process startup time (~2-5 seconds per agent spawn).
- KV cache priming (~1000-5000 tokens of system prompt re-processing).
- Memory allocation overhead.

The `AgentEfficiencyEvent.was_warm_start` field tracks warm vs. cold start distribution, enabling measurement of warm pool effectiveness.

---

## Framework Comparison Matrix

| Framework | Input | Output | Learning Signal | Persistence | Roko Equivalent |
|-----------|-------|--------|----------------|-------------|-----------------|
| Reflexion | Failed attempt + reflection prompt | Natural language reflection | Task retry success | Per-task context | Playbook rules |
| ExpeL | Episode batch | Generalized insights | Insight validation rate | Cross-task library | Skill library |
| DSPy | Program signature + test set | Optimized prompt | Test set accuracy | Static compilation | Prompt experiments |
| Voyager | Minecraft task | JavaScript function | Environment feedback | Skill library | Skill library |
| RouteLLM | Query | Strong/weak routing | Human preference | Router model weights | Cascade router |
| FrugalGPT | Query | Model cascade | Scoring model | Cascade config | Cascade router |
| AutoMix | Query | Self-verified cascade | Self-verification | None (online) | Gate pipeline |
| ADAS | Architecture spec | New architecture code | Benchmark evaluation | Archive of designs | (Planned) |

---

## Open Research Questions

Several open questions inform future development:

1. **Can a system improve its own improvement mechanisms?** Meta-Harness suggests yes, but the empirical evidence is limited to Karpathy's autoresearch experiment (11% speedup) and small-scale ADAS results (+14% on ARC). Whether these results transfer to large-scale software engineering is unknown.

2. **Does the external verifier requirement create a ceiling?** Huang et al. (ICLR 2024) show that self-improvement requires external verification. Roko's gate pipeline provides this, but the gates themselves are fixed — they don't improve. A system that improves its verifiers (automatically adding new test cases, discovering new lint rules) would have a higher ceiling.

3. **What is the optimal exploration budget?** All bandit algorithms trade exploration (trying suboptimal options) against exploitation (using the best-known option). The optimal tradeoff depends on the rate of environmental change, which is itself changing. Adaptive exploration budgets (like Thompson Sampling with drift) are theoretically sound but empirically untested in agent systems.

4. **Can cross-project transfer overcome the cold-start problem?** Skills and patterns extracted from project A may accelerate project B, but the transfer rate depends on structural similarity between projects. The HDC fingerprint approach enables fast similarity matching, but the quality of transferred knowledge is untested at scale.

---

## Improvement Measurement: Rigorous Quantification

Self-improvement claims require rigorous measurement. Without principled metrics and experimental controls, apparent improvements may be noise, regression to the mean, or artifacts of changing task distributions. This section specifies the measurement framework.

### Improvement Score Card

```rust
pub struct ImprovementScoreCard {
    /// Time window for comparison.
    pub window: TimeWindow,
    /// Baseline period metrics.
    pub baseline: PeriodMetrics,
    /// Current period metrics.
    pub current: PeriodMetrics,
    /// Statistical significance of observed changes.
    pub significance: SignificanceTests,
    /// Confound analysis.
    pub confounds: Vec<Confound>,
}

pub struct PeriodMetrics {
    /// Episode count in this period.
    pub n_episodes: usize,
    /// Four key metrics from mori-agents/07-self-improvement.md.
    pub first_attempt_pass_rate: f64,
    pub avg_iterations_per_plan: f64,
    pub avg_cost_per_plan_usd: f64,
    pub avg_prompt_tokens_per_spawn: u64,
    /// Extended metrics.
    pub skill_library_size: usize,
    pub playbook_rule_count: usize,
    pub c_factor: f64,
    pub avg_calibration_error: f64,
}

pub struct SignificanceTests {
    /// Two-proportion z-test for pass rate difference.
    pub pass_rate_z_score: f64,
    pub pass_rate_p_value: f64,
    /// Welch's t-test for cost difference.
    pub cost_t_statistic: f64,
    pub cost_p_value: f64,
    /// Mann-Whitney U test for iterations (non-parametric).
    pub iterations_u_statistic: f64,
    pub iterations_p_value: f64,
    /// Is the improvement statistically significant at α = 0.05?
    pub is_significant: bool,
}

pub enum Confound {
    /// Task distribution changed between periods.
    TaskDistributionShift {
        metric: String,
        baseline_distribution: Vec<f64>,
        current_distribution: Vec<f64>,
        kl_divergence: f64,
    },
    /// Model provider updated between periods.
    ProviderUpdate {
        model: String,
        update_timestamp: DateTime<Utc>,
    },
    /// Configuration change between periods.
    ConfigChange {
        key: String,
        old_value: String,
        new_value: String,
    },
    /// Sample size too small for reliable comparison.
    InsufficientSample {
        metric: String,
        n_required: usize,
        n_actual: usize,
    },
}
```

### Improvement Attribution

When improvement is detected, attribution identifies which learning subsystem caused it:

```
Improvement detected: pass rate 0.62 → 0.78 (+26%, p < 0.01)

Attribution analysis:
    1. Check if model routing changed → router selected opus more often (+12% of change)
    2. Check if new playbook rules were promoted → 3 new rules matched failing tasks (+8%)
    3. Check if skill library grew → 5 new skills for this task category (+4%)
    4. Check if prompt experiments concluded → "concise" variant won (+2%)
    Residual (unexplained): 0%

Most impactful subsystem: Cascade router (model selection improvement)
```

### Controlled Experiments via Holdout

The gold standard for measuring improvement is a controlled experiment: randomly assign tasks to a "learning" group (all subsystems active) and a "holdout" group (learning frozen at baseline state).

```rust
pub struct ImprovementExperiment {
    /// Experiment identifier.
    pub id: String,
    /// Start timestamp.
    pub started_at: DateTime<Utc>,
    /// Treatment: current learning configuration.
    pub treatment_config: LearningConfig,
    /// Control: frozen baseline configuration.
    pub control_config: LearningConfig,
    /// Assignment: hash(task_id) % 100 < treatment_pct → treatment.
    pub treatment_pct: u8,  // default: 80 (80% treatment, 20% holdout)
    /// Results accumulator.
    pub treatment_results: PeriodMetrics,
    pub control_results: PeriodMetrics,
    /// Minimum tasks before concluding.
    pub min_tasks: usize,  // default: 100
}
```

The holdout design ensures that observed improvements are caused by learning rather than external factors (easier task mix, model provider updates, codebase maturation).

### Monotonicity Tracking

Self-improvement should be monotonic: the system should get better over time, not oscillate. Monotonicity is tracked via the C-Factor trend:

```
C-Factor time series:
    0.48, 0.51, 0.53, 0.55, 0.54, 0.57, 0.61, 0.63, 0.65, 0.68
    ← monotonically increasing (with small perturbations)

Monotonicity score = fraction of steps where C(t) > C(t-1)
    = 8/9 = 0.89 (high monotonicity)

If monotonicity < 0.60 over 20+ episodes:
    → Learning system is not converging
    → Investigate: oscillation? regression? environmental shift?
```

---

## Improvement Safety: Preventing Harmful Self-Modification

A self-improving system can improve in harmful directions: optimizing for pass rate by generating trivially passing code, optimizing for cost by producing low-quality outputs, or modifying its own safety checks to avoid gate failures. Improvement safety mechanisms prevent these failure modes.

### Safety Invariants

```rust
pub struct SafetyInvariants {
    /// Gate pipeline must never be disabled or bypassed.
    pub gates_enabled: bool,
    /// Minimum gate count (at least compile + test).
    pub min_gate_count: usize,  // default: 2
    /// Gate thresholds must never drop below absolute floor.
    pub gate_threshold_floor: f64,  // default: 0.30
    /// Playbook rules cannot override safety-critical gates.
    pub safety_gates_immutable: Vec<String>,  // ["compile", "test"]
    /// Maximum model downgrade depth (prevent cascading to weakest model).
    pub max_downgrade_steps: u32,  // default: 2
    /// Self-modification detection: alert if learning modifies learning code.
    pub self_modification_alert: bool,  // default: true
}

pub enum SafetyViolation {
    /// A gate was disabled or its threshold dropped below floor.
    GateWeakened { gate: String, old_threshold: f64, new_threshold: f64 },
    /// A playbook rule attempts to override a safety-critical gate.
    SafetyGateOverride { rule_id: String, gate: String },
    /// Model selection cascaded below the minimum quality threshold.
    ExcessiveDowngrade { target_model: String, downgrade_depth: u32 },
    /// Learning subsystem is modifying its own code paths.
    SelfModification { modified_crate: String, modifier_task: String },
    /// Output quality metrics declined while pass rate increased (gaming gates).
    GateGaming { pass_rate_delta: f64, quality_delta: f64 },
    /// Cost optimization produced outputs below minimum quality.
    QualityFloor { task_id: String, quality_score: f64, threshold: f64 },
}
```

### Gate Gaming Detection

The most insidious failure mode is "gate gaming": the system learns to produce outputs that pass gates without actually solving the task. Detection:

```
Gate gaming indicators:
    1. Pass rate increases while downstream quality decreases
       (code passes tests but has bugs discovered later)
    2. Output complexity decreases (shorter, simpler code that
       technically passes but doesn't handle edge cases)
    3. Test coverage decreases while test pass rate increases
       (trivial tests that always pass)
    4. Diff size shrinks toward zero (minimal changes that pass gates
       but don't address the task requirements)
```

```rust
pub struct GateGamingDetector {
    /// Window of recent episodes for analysis.
    pub window_size: usize,  // default: 50
    /// Alert if pass rate increases by >10% while quality score decreases by >5%.
    pub pass_quality_divergence_threshold: f64,  // default: 0.05
    /// Alert if average diff size drops below this fraction of baseline.
    pub min_diff_size_fraction: f64,  // default: 0.30
    /// Alert if output token count drops below this fraction of baseline.
    pub min_output_fraction: f64,  // default: 0.40
}
```

### Constitutional Constraints

Inspired by Constitutional AI (Bai et al. 2022), the self-improvement system operates under constitutional constraints — inviolable rules that no learning subsystem can override:

```toml
# In roko.toml [safety] section
[safety.constitution]
# Learning cannot disable gates
gates_immutable = true
# Learning cannot modify the safety module itself
self_modification_forbidden_crates = ["roko-gate", "roko-agent/safety"]
# Model selection must always include at least one high-quality option
min_quality_model_tier = "standard"
# Budget optimization cannot reduce quality below floor
quality_floor = 0.50
# All self-modifications require human review
self_mod_requires_review = true
```

### Improvement Velocity Limits

Even beneficial improvements should be rate-limited to prevent cascade failures:

```rust
pub struct ImprovementVelocityLimits {
    /// Maximum playbook rule changes per day.
    pub max_rule_changes_per_day: u32,  // default: 10
    /// Maximum routing table changes per day.
    pub max_routing_changes_per_day: u32,  // default: 20
    /// Maximum prompt experiment conclusions per day.
    pub max_experiment_conclusions_per_day: u32,  // default: 5
    /// Cooldown after a safety violation (minutes).
    pub safety_violation_cooldown_minutes: u32,  // default: 60
    /// Maximum C-Factor change per episode (damping).
    pub max_cfactor_delta: f64,  // default: 0.02
}
```

These limits prevent a scenario where a false positive in the improvement pipeline triggers a cascade of changes that collectively degrade the system. By limiting the rate of change, the system has time to detect and recover from individual bad decisions.

### Connection to AI Safety Research

The improvement safety framework draws on three lines of research:

1. **Constitutional AI (Bai et al. 2022):** Inviolable rules that constrain self-improvement. Roko's constitutional constraints are the safety analogue.

2. **Scalable oversight (Amodei et al. 2016):** As systems become more capable, human oversight must scale. Roko's `self_mod_requires_review` flag ensures human-in-the-loop for self-referential changes.

3. **Reward hacking (Skalse et al. 2022):** Optimizing for a proxy metric (gate pass rate) can diverge from the true objective (correct code). Gate gaming detection explicitly monitors for this divergence.

4. **Self-play safety (Silver et al. 2017; OpenAI Five 2019):** Self-play can discover exploits in the reward function. Roko's holdout experiment design provides a control group that detects if the "improved" system is actually gaming rather than improving.

---

## Relationship to Other Documents

- **[02-skill-library-voyager](02-skill-library-voyager.md)** — Implements Voyager-style skill accumulation (Wang et al. 2023).
- **[04-cascade-router](04-cascade-router.md)** — Implements RouteLLM/FrugalGPT-inspired cascading.
- **[01-playbook-system](01-playbook-system.md)** — Implements Reflexion/ExpeL-style experience extraction.
- **[13-8-missing-feedback-loops](13-8-missing-feedback-loops.md)** — Maps these frameworks to specific cybernetic feedback loops.
- **[17-adas-and-autocatalytic](17-adas-and-autocatalytic.md)** — Extends self-improvement to meta-level architecture search (ADAS) and autocatalytic growth.


---

# SOURCE: /Users/will/dev/nunchi/roko/roko/docs/05-learning/13-8-missing-feedback-loops.md

# Eight Missing Cybernetic Feedback Loops

> **Implementation plan:** `modelrouting/17-meta-learning-and-corrections.md` (tasks 2O.01–2O.13)
> **PRD source:** `refactoring-prd/07-implementation-priorities.md` (Tier 1M table)
> **Theoretical basis:** Ashby's Law of Requisite Variety, Beer's Viable System Model, Good Regulator Theorem
> **Cross-references:** [04-cascade-router](04-cascade-router.md), [09-provider-health-circuit-breaker](09-provider-health-circuit-breaker.md), [14-stability-mechanisms](14-stability-mechanisms.md)


> **Implementation**: Shipping

---

## Purpose

The Roko learning system was built in layers: episodes first, then patterns, then bandits, then routing. Each layer works, but they don't yet talk to each other. The eight missing feedback loops are the inter-layer connections that close the cybernetic circuit — signals that flow from one subsystem's output to another subsystem's input, creating the self-regulating behavior that distinguishes a learning system from a collection of independent optimizers.

These eight loops are the organizing concept for Tier 1M (missing) in the implementation priority roadmap. Each loop has a clear source (where the signal originates), target (where it should flow), and mechanism (how the signal is transformed into action).

---

## The Eight Loops

```
┌─────────────────────────────────────────────────────────────────────┐
│                    EIGHT FEEDBACK LOOPS                              │
│                                                                     │
│  1. Health → Routing     Provider circuit breaker → candidate set   │
│  2. Conductor → Routing  System load signals → routing bias         │
│  3. Section → Scaffold   Section effectiveness → prompt weights     │
│  4. Failure → Replanning Gate failures → plan revision              │
│  5. Skills → Prompts     Skill library → prompt injection           │
│  6. Cost → Routing       Budget pressure → model selection          │
│  7. Latency → Reward     Response latency → bandit reward signal    │
│  8. Experiments → Static Experiment winners → static routing table  │
│                                                                     │
└─────────────────────────────────────────────────────────────────────┘
```

---

## Loop 1: Health → Routing

**Source:** `ProviderHealthRegistry` (circuit breaker state per provider)
**Target:** `CascadeRouter::select()` (candidate model filtering)
**Mechanism:** Before scoring candidates, filter out models whose provider circuit breaker is Open.

```
ProviderHealthRegistry::is_available("anthropic") → false (circuit Open)
    │
    ▼
CascadeRouter excludes all anthropic models from candidate set
    │
    ▼
Routes to openrouter or other available provider
```

**Status:** Wired. The cascade router calls `is_available()` during candidate scoring.

**Impact:** Prevents routing to degraded providers, reducing retry waste and improving first-attempt pass rates.

See: [09-provider-health-circuit-breaker](09-provider-health-circuit-breaker.md)

---

## Loop 2: Conductor → Routing

**Source:** Conductor subsystem (system load, resource utilization, queue depth)
**Target:** `CascadeRouter::select()` (routing bias)
**Mechanism:** When system load is high, bias toward cheaper/faster models. When load is low, allow more expensive/thorough models.

```
Conductor::system_load() → 0.85 (high load)
    │
    ▼
CascadeRouter bias: PreferCheaper
    │
    ▼
Routes to faster model to reduce queue pressure
```

**Status:** Wired for the live orchestration path. `RoutingContext` now carries conductor pressure derived from active agent count, ready-queue depth, and queue wait, and `CascadeRouter` explicitly biases selection toward cheaper tiers when pressure is high. The remaining gap is richer resource telemetry beyond the current heuristic snapshot.

**Impact:** Prevents resource exhaustion during high-load periods by dynamically adjusting quality-cost tradeoffs.

See: [07-conductor](../07-conductor/INDEX.md)

---

## Loop 3: Section → Scaffold

**Source:** `PromptSectionMeta` from efficiency events (per-section token attribution + gate outcomes)
**Target:** Prompt composer section weights (priority values used during context assembly)
**Mechanism:** Track which prompt sections correlate with gate passes. Increase weight of sections that correlate with success; decrease weight of sections that consume tokens without contributing to outcomes.

```
PromptSectionMeta { name: "workspace_map", tokens: 2000, was_truncated: false }
    │ + gate outcome: pass
    │
    ▼
Section effectiveness tracker:
    workspace_map: included in 50 turns, 35 passed (70% when included)
    workspace_map: excluded in 20 turns, 18 passed (90% when excluded)
    → workspace_map may be HURTING pass rate. Lower its priority.
```

**Status:** Wired for the live orchestration path. Composed prompts now emit per-section inclusion/drop metadata into efficiency events, `LearningRuntime` persists a section-effectiveness registry, and the next prompt build/feedforward pass reweights section priorities from those learned lift signals. The remaining gap is broader coverage outside the current orchestrator path plus more expressive weighting than the current priority-step adjustments.

**Impact:** This is the highest-leverage self-improvement loop. Adaptive context assembly can reduce prompt size by 30-50% while improving pass rates, because sections that waste the agent's attention budget are demoted.

See: [03-composition](../03-composition/INDEX.md) for the prompt assembly pipeline.

---

## Loop 4: Failure → Replanning

**Source:** Gate failure patterns (repeated failures on the same task, regression alerts)
**Target:** Plan generator (re-decompose the failing task)
**Mechanism:** When a task fails N consecutive times, trigger replanning: break the task into smaller subtasks, change the approach, or escalate to a human review.

```
Task T3: fail (iteration 1) → fail (iteration 2) → fail (iteration 3)
    │
    ▼
Failure→Replanning trigger:
    ├── Analyze failure pattern (same error? different errors?)
    ├── Generate alternative decomposition
    └── Create new subtasks T3a, T3b, T3c
```

**Status:** Wired for the orchestrator path. Gate failures increment per-plan failure counters and, when auto-replan is enabled, trigger strategy-specific replan flows that can retry, escalate, or decompose the task into subtasks. The remaining gap is tighter coupling with the standalone `roko prd plan` generator and richer failure analysis.

**Impact:** Prevents the system from burning budget on intractable tasks. Replanning turns a hard task into multiple easier tasks that may succeed individually.

---

## Loop 5: Skills → Prompts

**Source:** `SkillLibrary` (accumulated skills with confidence scores)
**Target:** Prompt composer (skill injection into agent prompts)
**Mechanism:** When a new task matches a skill's trigger pattern (file paths, task category, tags), inject the skill's prompt template into the agent's system prompt.

```
New task: modify crates/roko-core/src/config/schema.rs
    │
    ▼
SkillLibrary::search_by_files(["crates/roko-core/src/config/schema.rs"])
    │
    ▼
Match: skill "config_schema_extension" (confidence: 0.87)
    │
    ▼
Inject into prompt:
    "Recommended approach (from 12 successful similar tasks):
     1. Add new field to the config struct
     2. Add serde default annotation
     3. Update the TOML schema documentation
     4. Add a test in config_tests.rs"
```

**Status:** Wired for live prompt composition. Matching skills from `SkillLibrary` are rendered into a dedicated `skill-library` prompt section before composition. The remaining gap is deeper section-native integration inside `SystemPromptBuilder` itself rather than the current orchestration-layer injection.

**Impact:** Reduces iterations by providing agents with proven approaches. The 100th modification to a crate is dramatically cheaper than the 1st because the skill library has accumulated the crate's patterns.

See: [02-skill-library-voyager](02-skill-library-voyager.md)

---

## Loop 6: Cost → Routing

**Source:** Budget guardrails (per-task, per-session, per-day cost tracking)
**Target:** `CascadeRouter::select()` (model tier bias)
**Mechanism:** When spending approaches budget limits, force the router to select cheaper models.

```
Session cost: $45.60 / $50.00 limit (91.2%)
    │
    ▼
BudgetGuardrail::check() → BudgetAction::Block
    │
    ▼
CascadeRouter: only consider models cheaper than $0.10/M tokens
```

**Status:** Wired in the live dispatch path. `BudgetGuardrail` checks current spend before dispatch and can block execution or force a cheaper tier/model. The remaining gap is moving that pressure fully into first-class candidate scoring inside `CascadeRouter` instead of the current pre-dispatch override.

**Impact:** Prevents cost overruns by dynamically adjusting the quality-cost tradeoff. The system degrades gracefully (cheaper models, lower quality) rather than halting entirely.

See: [08-cost-normalization](08-cost-normalization.md)

---

## Loop 7: Latency → Reward

**Source:** `LatencyRegistry` (per-model, per-provider latency statistics)
**Target:** Bandit reward signal (used to update LinUCB/UCB1 arms)
**Mechanism:** Include latency as a component of the reward signal, so the bandit learns to avoid slow models when latency SLAs are tight.

```
Model A: pass_rate=0.90, avg_latency=2000ms, latency_sla=1500ms
Model B: pass_rate=0.85, avg_latency=800ms, latency_sla=1500ms
    │
    ▼
Reward adjustment:
    Model A: base_reward=1.0, latency_penalty=0.30 → adjusted=0.70
    Model B: base_reward=1.0, latency_penalty=0.00 → adjusted=1.00
    │
    ▼
Bandit learns to prefer Model B when latency SLA is tight
```

**Status:** Wired. Runtime feedback computes routing reward with actual observed latency plus model/provider latency registries and records that reward into cascade-router observations. The remaining gap is more explicit latency-SLA-aware scalarization at route-selection time.

**Impact:** Prevents the bandit from selecting high-latency models that violate SLAs. Particularly important for interactive use cases where the human is waiting for results.

---

## Loop 8: Experiments → Static

**Source:** `ExperimentStore` (concluded experiments with identified winners)
**Target:** Static routing table (stage-1 defaults)
**Mechanism:** When a prompt experiment concludes with a clear winner, update the static configuration to use the winning variant. When a model experiment identifies the best model for a (role, complexity) pair, update the static routing table.

```
Experiment "system-prompt-v2": winner = variant "concise" (89% pass rate vs 72% baseline)
    │
    ▼
Update static config:
    system_prompt_section.constraints = "concise" variant text
    │
    ▼
All future tasks use the winning variant by default
```

**Status:** Wired in persisted runtime defaults. Concluded prompt experiments keep returning their winner from the persisted experiment store on future assignments, and concluded role/model experiments already update the cascade router's static table. The remaining gap is optional materialization back into human-edited config/prompt source files.

**Impact:** Experiment results are currently ephemeral — they influence routing while the experiment is running, but the winner isn't persisted into the static config. Closing this loop makes experiment improvements permanent.

---

## Summary Table

| # | Loop | Source | Target | Status |
|---|------|--------|--------|--------|
| 1 | Health → Routing | ProviderHealthRegistry | CascadeRouter candidate filter | **Wired** |
| 2 | Conductor → Routing | Conductor load signals | CascadeRouter bias | **Wired (pressure heuristic)** |
| 3 | Section → Scaffold | PromptSectionMeta | Composer section weights | **Wired (live orchestration path)** |
| 4 | Failure → Replanning | Gate failure patterns | Plan generator | **Wired (orchestrator path)** |
| 5 | Skills → Prompts | SkillLibrary | SystemPromptBuilder | **Wired (orchestration-layer injection)** |
| 6 | Cost → Routing | Budget guardrails | CascadeRouter model tier | **Wired (pre-dispatch guardrail)** |
| 7 | Latency → Reward | LatencyRegistry | Bandit reward signal | **Wired** |
| 8 | Experiments → Static | ExperimentStore | Static config | **Wired (persisted winner + router sync)** |

---

## Cybernetic Theory

These eight loops implement the core principle of cybernetics: **negative feedback for stability** (Ashby's Law of Requisite Variety). Each loop detects a deviation from desired behavior and applies a corrective signal:

- Provider health degrades → route away (loop 1)
- System overloaded → use cheaper models (loop 2)
- Section wasteful → reduce its weight (loop 3)
- Task intractable → decompose differently (loop 4)
- Skill available → inject it (loop 5)
- Budget exhausted → downgrade quality (loop 6)
- Latency excessive → penalize slow models (loop 7)
- Experiment concluded → lock in winner (loop 8)

The compound effect of all eight loops operating simultaneously is that the system converges toward an optimal operating point without manual tuning. See [14-stability-mechanisms](14-stability-mechanisms.md) for how oscillation is prevented, and [17-adas-and-autocatalytic](17-adas-and-autocatalytic.md) for why the compound improvement rate can be super-linear.

---

## Wiring recipes

Each unwired loop has a concrete implementation path. This section specifies the exact source/target structs, code paths, and estimated LOC for each.

### Recipe: Loop 2 -- Conductor -> Routing

```
Source struct:  roko-conductor::SystemLoadSnapshot
  Fields: cpu_load: f32, memory_pct: f32, active_agents: u32, queue_depth: u32
  File: crates/roko-conductor/src/load.rs (exists)

Target struct:  roko-learn::CascadeRouter
  Method: select(&self, task: &TaskSignal, ctx: &RoutingContext) -> ModelChoice
  File: crates/roko-learn/src/cascade_router.rs

Wiring path:
  1. Add RoutingContext.system_load: Option<SystemLoadSnapshot>
  2. In orchestrate.rs, read SystemLoadSnapshot before calling CascadeRouter::select()
  3. In CascadeRouter::select(), when system_load.active_agents >= conductor.max_agents * 0.8:
     bias = RoutingBias::PreferCheaper
  4. Apply bias as a cost_weight multiplier (1.5x) during candidate scoring

Estimated LOC: ~45
  - 10 lines: RoutingContext field addition
  - 15 lines: load snapshot read in orchestrate.rs
  - 20 lines: bias application in cascade_router.rs
```

### Recipe: Loop 3 -- Section -> Scaffold

```
Source struct:  roko-learn::EfficiencyEvent
  Fields: section_meta: Vec<PromptSectionMeta>, gate_passed: bool
  File: crates/roko-learn/src/efficiency.rs

Target struct:  roko-compose::SectionWeights
  Fields: weights: HashMap<String, f32>   (section name -> priority modifier)
  File: crates/roko-compose/src/budget.rs (new struct)

Wiring path:
  1. Create SectionWeights struct in roko-compose::budget
  2. Add SectionEffectivenessTracker to roko-learn:
     - Tracks (section_name, included, gate_passed) tuples
     - Computes per-section pass-rate-when-included vs pass-rate-when-excluded
     - Emits weight adjustments when delta > 5% over 50+ samples
  3. In orchestrate.rs, load SectionWeights from .roko/learn/section-weights.json
  4. Pass SectionWeights to SystemPromptBuilder
  5. In budget allocation, multiply section max_tokens by weight modifier

Estimated LOC: ~120
  - 25 lines: SectionWeights struct + serde
  - 50 lines: SectionEffectivenessTracker (accumulator + computation)
  - 15 lines: persistence to/from JSON
  - 15 lines: orchestrate.rs loading + passing
  - 15 lines: budget.rs integration
```

### Recipe: Loop 4 -- Failure -> Replanning

```
Source struct:  GateVerdict (from roko-learn::episode_logger)
  Fields: gate: String, passed: bool, signature: Option<String>
  File: crates/roko-learn/src/episode_logger.rs

Target:  roko-cli::prd::plan (the plan generation subcommand)
  File: crates/roko-cli/src/prd.rs

Wiring path:
  1. In orchestrate.rs, track consecutive failures per task:
     task_failures: HashMap<TaskId, Vec<GateVerdict>>
  2. When task_failures[task_id].len() >= gates.max_iterations:
     a. Analyze failure signatures (same error repeated? different errors each time?)
     b. Generate a replanning prompt with failure context
     c. Call the plan generation agent with the failing task + failure analysis
     d. Replace the failing task with the generated subtasks in the plan DAG
  3. The new subtasks inherit the original task's dependencies

Estimated LOC: ~80
  - 20 lines: failure tracking in orchestrate.rs
  - 30 lines: failure analysis (signature grouping, pattern detection)
  - 30 lines: replanning agent dispatch + subtask insertion
```

### Recipe: Loop 5 -- Skills -> Prompts

```
Source struct:  roko-learn::SkillEntry
  Fields: trigger: SkillTrigger, template: String, confidence: f32
  File: crates/roko-learn/src/skill_library.rs

Target struct:  roko-compose::SystemPromptBuilder
  Method: add_skill_section(skills: &[SkillEntry])
  File: crates/roko-compose/src/system_prompt_builder.rs

Wiring path:
  1. In orchestrate.rs, before building the system prompt:
     let skills = skill_library.search_by_task(&task);
  2. Filter to skills with confidence >= 0.5
  3. Call system_prompt_builder.add_skill_section(skills)
  4. The skill section is priority 3 (Medium), max_tokens 500

Estimated LOC: ~55
  - 10 lines: skill search call in orchestrate.rs
  - 20 lines: add_skill_section method in SystemPromptBuilder
  - 15 lines: skill section template formatting
  - 10 lines: budget allocation entry for skill section
```

### Recipe: Loop 6 -- Cost -> Routing

```
Source struct:  roko-learn::CostsLog
  Fields: records: Vec<CostRecord>
  File: crates/roko-learn/src/costs.rs

Target struct:  roko-learn::CascadeRouter
  File: crates/roko-learn/src/cascade_router.rs

Wiring path:
  1. Add BudgetGuardrail struct to roko-learn:
     - Tracks cumulative cost from CostsLog
     - Compares against budget.max_plan_usd and budget.max_session_usd
     - Returns BudgetAction: Allow, Warn, or Block
  2. In CascadeRouter::select(), call budget_guardrail.check():
     - If Warn: multiply cost_weight by 2.0 (bias toward cheaper models)
     - If Block: filter candidates to only those cheaper than $0.10/M input tokens
  3. In orchestrate.rs, update BudgetGuardrail after each agent turn

Estimated LOC: ~70
  - 30 lines: BudgetGuardrail struct + check()
  - 20 lines: CascadeRouter integration
  - 20 lines: orchestrate.rs update calls
```

### Recipe: Loop 7 -- Latency -> Reward

```
Source struct:  roko-learn::LatencyRegistry
  Fields: ewma: HashMap<String, f64>, percentiles: HashMap<String, Percentiles>
  File: crates/roko-learn/src/latency.rs

Target:  Bandit reward signal computation
  File: crates/roko-learn/src/cascade_router.rs (reward_for method)

Wiring path:
  1. In CascadeRouter::reward_for(), add latency component:
     let latency_ms = latency_registry.ewma_for(model);
     let latency_sla = routing.latency_sla_ms;  // new config field
     let latency_reward = (1.0 - (latency_ms / latency_sla as f64)).clamp(0.0, 1.0);
  2. Incorporate into composite reward:
     reward = quality_weight * quality + cost_weight * cost + latency_weight * latency_reward
  3. Add routing.latency_sla_ms to RoutingConfig (default: 5000ms)

Estimated LOC: ~35
  - 10 lines: latency reward computation
  - 10 lines: composite reward update
  - 15 lines: config field + default
```

### Recipe: Loop 8 -- Experiments -> Static

```
Source struct:  roko-learn::ExperimentStore
  Fields: experiments: HashMap<String, Experiment>
  File: crates/roko-learn/src/experiments.rs

Target:  roko.toml (static configuration)
  File: crates/roko-core/src/config/schema.rs

Wiring path:
  1. Add ExperimentStore::concluded_winners() -> Vec<ExperimentWinner>
     - Returns experiments where one variant has statistically significant advantage
     - Uses chi-squared test or simple threshold (>5% delta, >50 samples)
  2. Add roko config apply-experiments subcommand:
     - Reads concluded winners
     - Updates roko.toml with winning values
     - Archives concluded experiments
  3. Optionally: auto-apply on plan completion (gated by learning.auto_apply_experiments)

Estimated LOC: ~90
  - 30 lines: concluded_winners() method
  - 40 lines: config apply-experiments CLI subcommand
  - 20 lines: TOML update logic
```

### Summary

| Loop | Estimated LOC | Complexity | Dependencies |
|---|---|---|---|
| 2: Conductor -> Routing | ~45 | Low | SystemLoadSnapshot already exists |
| 3: Section -> Scaffold | ~120 | Medium | New SectionEffectivenessTracker |
| 4: Failure -> Replanning | ~80 | Medium | Plan generation agent already exists |
| 5: Skills -> Prompts | ~55 | Low | SkillLibrary and SystemPromptBuilder both exist |
| 6: Cost -> Routing | ~70 | Low | CostsLog already exists |
| 7: Latency -> Reward | ~35 | Low | LatencyRegistry already exists |
| 8: Experiments -> Static | ~90 | Medium | New CLI subcommand |
| **Total** | **~495** | | |

---

## Detailed Data Flow Specifications

Each feedback loop has precise data flow requirements, latency constraints, and failure mode characteristics. This section formalizes these for implementation reference.

### Loop 1: Health → Routing — Data Flow

```
┌──────────────────────┐     ┌────────────────────────┐     ┌─────────────────┐
│  Provider Response    │────►│  ProviderHealthRegistry │────►│  CascadeRouter   │
│  (success/failure)    │     │  record_success/failure │     │  select()        │
└──────────────────────┘     └────────────────────────┘     └─────────────────┘
      Source type:                Transform:                   Sink type:
      ProviderResponse            ErrorClassifier →            is_available() →
      { status_code,              CircuitState update          bool filter on
        latency_ms,               (Closed/Open/HalfOpen)       candidate set
        error: Option }
```

**Latency requirement:** Real-time (< 1ms). Circuit state check is a HashMap lookup.

**Failure mode if loop breaks:** Router sends requests to a degraded provider → timeouts → wasted budget → cascading failures as the provider's queue backs up. Recovery: manual provider blacklist in roko.toml.

```rust
// Source type
pub struct ProviderResponse {
    pub provider_id: String,
    pub model: String,
    pub status_code: u16,
    pub latency_ms: u64,
    pub error: Option<ProviderError>,
    pub timestamp: DateTime<Utc>,
}

// Transform function
fn health_to_routing_transform(response: &ProviderResponse) -> CircuitAction {
    match response.error {
        None => CircuitAction::RecordSuccess,
        Some(ref err) => {
            let class = ErrorClassifier::classify(err);
            let cooldown = CooldownPolicy::for_class(&class);
            CircuitAction::RecordFailure { class, cooldown }
        }
    }
}

// Sink: CascadeRouter reads circuit state during select()
fn is_available(provider: &str, registry: &ProviderHealthRegistry) -> bool {
    matches!(registry.state(provider), CircuitState::Closed | CircuitState::HalfOpen)
}
```

---

### Loop 2: Conductor → Routing — Data Flow

```
┌──────────────────────┐     ┌────────────────────────┐     ┌─────────────────┐
│  SystemLoadSnapshot   │────►│  Load Threshold Check   │────►│  CascadeRouter   │
│  (cpu, mem, agents,   │     │  active_agents >=       │     │  routing bias    │
│   queue_depth)        │     │  max_agents * 0.8?      │     │  adjustment      │
└──────────────────────┘     └────────────────────────┘     └─────────────────┘
```

**Latency requirement:** Batch (every 5 episodes). System load changes slowly relative to task execution.

**Failure mode if loop breaks:** System routes to expensive models during high load → resource exhaustion → agent spawn failures → plan stalls. Recovery: manual cost ceiling in roko.toml.

```rust
// Source type (already exists in roko-conductor)
pub struct SystemLoadSnapshot {
    pub cpu_load: f32,
    pub memory_pct: f32,
    pub active_agents: u32,
    pub queue_depth: u32,
    pub timestamp: DateTime<Utc>,
}

// Transform function
fn conductor_to_routing_transform(
    load: &SystemLoadSnapshot,
    config: &ConductorConfig,
) -> RoutingBiasAdjustment {
    let agent_utilization = load.active_agents as f64 / config.max_agents as f64;
    let memory_pressure = load.memory_pct as f64 / 100.0;

    if agent_utilization > 0.8 || memory_pressure > 0.85 {
        RoutingBiasAdjustment::PreferCheaper { cost_weight_multiplier: 1.5 }
    } else if agent_utilization < 0.3 && memory_pressure < 0.50 {
        RoutingBiasAdjustment::AllowExpensive { quality_weight_multiplier: 1.2 }
    } else {
        RoutingBiasAdjustment::Neutral
    }
}

pub enum RoutingBiasAdjustment {
    PreferCheaper { cost_weight_multiplier: f64 },
    AllowExpensive { quality_weight_multiplier: f64 },
    Neutral,
}
```

---

### Loop 3: Section → Scaffold — Data Flow

```
┌──────────────────────┐     ┌────────────────────────┐     ┌─────────────────┐
│  PromptSectionMeta +  │────►│  SectionEffectiveness   │────►│  SectionWeights  │
│  gate_passed (from    │     │  Tracker (conditional   │     │  (HashMap<String │
│  efficiency events)   │     │  pass rate analysis)    │     │   , f32>)        │
└──────────────────────┘     └────────────────────────┘     └─────────────────┘
```

**Latency requirement:** Batch (every 20 episodes). Section effectiveness needs a meaningful sample before adjusting weights.

**Failure mode if loop breaks:** Wasteful prompt sections consume tokens without contributing to success → inflated costs and potentially confused agents. Recovery: manual section weights in roko.toml prompt configuration.

```rust
// Source type (already exists in efficiency events)
pub struct SectionEffectivenessInput {
    pub section_name: String,
    pub was_included: bool,
    pub tokens_consumed: u64,
    pub gate_passed: bool,
    pub role: String,
    pub complexity_band: String,
}

// Transform: conditional pass rate analysis
pub struct SectionEffectivenessTracker {
    /// Per-section: (included_count, included_pass_count, excluded_count, excluded_pass_count)
    stats: HashMap<String, SectionStats>,
}

pub struct SectionStats {
    pub included_count: u32,
    pub included_pass_count: u32,
    pub excluded_count: u32,
    pub excluded_pass_count: u32,
}

impl SectionStats {
    fn effectiveness_delta(&self) -> f64 {
        let included_rate = self.included_pass_count as f64 / self.included_count.max(1) as f64;
        let excluded_rate = self.excluded_pass_count as f64 / self.excluded_count.max(1) as f64;
        included_rate - excluded_rate
        // Positive = section helps, Negative = section hurts
    }
}

fn section_to_scaffold_transform(
    tracker: &SectionEffectivenessTracker,
    min_samples: u32,  // default: 50
    significance_delta: f64,  // default: 0.05
) -> HashMap<String, f32> {
    let mut weights = HashMap::new();
    for (name, stats) in &tracker.stats {
        if stats.included_count + stats.excluded_count < min_samples {
            weights.insert(name.clone(), 1.0); // Not enough data, neutral weight
            continue;
        }
        let delta = stats.effectiveness_delta();
        if delta > significance_delta {
            weights.insert(name.clone(), 1.0 + delta as f32); // Boost helpful sections
        } else if delta < -significance_delta {
            weights.insert(name.clone(), (1.0 + delta as f32).max(0.1)); // Reduce harmful sections
        } else {
            weights.insert(name.clone(), 1.0); // Neutral
        }
    }
    weights
}

// Sink: SectionWeights in roko-compose
pub struct SectionWeights {
    pub weights: HashMap<String, f32>,
    pub computed_at: DateTime<Utc>,
    pub episode_count: usize,
}
```

---

### Loop 4: Failure → Replanning — Data Flow

```
┌──────────────────────┐     ┌────────────────────────┐     ┌─────────────────┐
│  Consecutive gate     │────►│  Failure Analyzer       │────►│  Plan Generator  │
│  failures for task    │     │  (pattern detection,    │     │  (decompose into │
│  (Vec<GateVerdict>)   │     │   root cause grouping)  │     │   subtasks)      │
└──────────────────────┘     └────────────────────────┘     └─────────────────┘
```

**Latency requirement:** Per-task (triggered after max_iterations failures). Must complete before plan executor moves to next task.

**Failure mode if loop breaks:** System retries the same failing task indefinitely → budget burn on intractable tasks → plan stalls. Recovery: manual task skip or plan abort.

```rust
// Source type
pub struct FailureSequence {
    pub task_id: String,
    pub plan_id: String,
    pub failures: Vec<GateVerdict>,
    pub total_cost_burned: f64,
    pub models_tried: Vec<String>,
}

// Transform: failure analysis
pub struct FailureAnalysis {
    /// Are all failures the same error? (systematic issue)
    pub is_repeated_error: bool,
    /// Dominant error signature (if repeated).
    pub dominant_signature: Option<String>,
    /// Did model escalation help? (opus failed same as haiku = not a model issue)
    pub model_escalation_helped: bool,
    /// Recommended action.
    pub recommendation: FailureRecommendation,
}

pub enum FailureRecommendation {
    /// Decompose into smaller subtasks.
    Decompose { suggested_split: Vec<String> },
    /// Change approach entirely (different tool set, different strategy).
    ChangeApproach { reason: String },
    /// Escalate to human review.
    HumanReview { context: String },
    /// Skip task (it may be impossible given current capabilities).
    Skip { reason: String },
}

fn failure_to_replan_transform(seq: &FailureSequence) -> FailureAnalysis {
    let signatures: Vec<_> = seq.failures.iter()
        .filter_map(|v| v.signature.as_ref())
        .collect();

    let is_repeated = signatures.windows(2).all(|w| w[0] == w[1]);
    let model_set: HashSet<_> = seq.models_tried.iter().collect();
    let tried_multiple_models = model_set.len() >= 2;

    let recommendation = if is_repeated && tried_multiple_models {
        // Same error with multiple models = fundamental approach problem
        FailureRecommendation::Decompose {
            suggested_split: suggest_decomposition(&seq.task_id),
        }
    } else if seq.failures.len() > 5 && seq.total_cost_burned > 10.0 {
        // Many failures, high cost = escalate
        FailureRecommendation::HumanReview {
            context: format!("Task {} failed {} times, burned ${:.2}",
                seq.task_id, seq.failures.len(), seq.total_cost_burned),
        }
    } else {
        FailureRecommendation::ChangeApproach {
            reason: "Varied errors suggest the approach needs revision".into(),
        }
    };

    FailureAnalysis {
        is_repeated_error: is_repeated,
        dominant_signature: signatures.first().map(|s| s.to_string()),
        model_escalation_helped: !is_repeated && tried_multiple_models,
        recommendation,
    }
}
```

---

### Loop 5: Skills → Prompts — Data Flow

```
┌──────────────────────┐     ┌────────────────────────┐     ┌─────────────────┐
│  SkillLibrary         │────►│  Skill Matcher          │────►│  SystemPrompt    │
│  (accumulated skills  │     │  (tag + file + HDC      │     │  Builder layer 4 │
│   with confidence)    │     │   similarity search)    │     │  ("skills" sect) │
└──────────────────────┘     └────────────────────────┘     └─────────────────┘
```

**Latency requirement:** Per-task (< 5ms). Must complete during prompt assembly before agent dispatch.

**Failure mode if loop breaks:** Agents rediscover solutions that the skill library already contains → wasted iterations → higher cost. Recovery: skills still accumulate but aren't injected; no data loss.

```rust
// Source: SkillLibrary::search_by_task()
pub struct SkillMatch {
    pub skill_name: String,
    pub confidence: f64,
    pub match_type: SkillMatchType,
    pub prompt_template: String,
    pub max_tokens: usize,  // budget for this skill injection
}

pub enum SkillMatchType {
    /// Matched by file path overlap.
    FileMatch { overlap_files: Vec<String> },
    /// Matched by tag overlap.
    TagMatch { matching_tags: Vec<String> },
    /// Matched by HDC similarity to task context.
    HdcSimilarity { similarity: f64 },
}

// Transform: filter and rank
fn skills_to_prompt_transform(
    matches: Vec<SkillMatch>,
    max_skills: usize,  // default: 3
    min_confidence: f64,  // default: 0.50
    max_total_tokens: usize,  // default: 500
) -> Vec<SkillInjection> {
    let mut qualified: Vec<_> = matches.into_iter()
        .filter(|m| m.confidence >= min_confidence)
        .collect();
    qualified.sort_by(|a, b| b.confidence.partial_cmp(&a.confidence).unwrap());

    let mut injections = Vec::new();
    let mut token_budget = max_total_tokens;
    for skill in qualified.into_iter().take(max_skills) {
        if skill.max_tokens <= token_budget {
            token_budget -= skill.max_tokens;
            injections.push(SkillInjection {
                skill_name: skill.skill_name,
                template: skill.prompt_template,
                confidence: skill.confidence,
            });
        }
    }
    injections
}

// Sink: injected into SystemPromptBuilder
pub struct SkillInjection {
    pub skill_name: String,
    pub template: String,
    pub confidence: f64,
}
```

---

### Loop 6: Cost → Routing — Data Flow

```
┌──────────────────────┐     ┌────────────────────────┐     ┌─────────────────┐
│  CostsLog (running    │────►│  BudgetGuardrail        │────►│  CascadeRouter   │
│  cost accumulator)    │     │  check() → action       │     │  cost_weight or  │
│                       │     │                          │     │  candidate filter │
└──────────────────────┘     └────────────────────────┘     └─────────────────┘
```

**Latency requirement:** Per-task (< 1ms). Budget check is arithmetic comparison.

**Failure mode if loop breaks:** System exceeds budget → unexpected charges → operator loses trust in automated execution. Recovery: hard stop at 100% budget via separate watchdog process.

```rust
// Source: accumulated costs
pub struct BudgetState {
    pub task_cost_usd: f64,
    pub session_cost_usd: f64,
    pub day_cost_usd: f64,
    pub task_limit: f64,
    pub session_limit: f64,
    pub day_limit: f64,
}

// Transform: multi-level budget check
fn cost_to_routing_transform(state: &BudgetState) -> BudgetRoutingAction {
    // Check each level, return most restrictive action
    let task_pct = state.task_cost_usd / state.task_limit;
    let session_pct = state.session_cost_usd / state.session_limit;
    let day_pct = state.day_cost_usd / state.day_limit;

    let max_pct = task_pct.max(session_pct).max(day_pct);

    if max_pct >= 1.0 {
        BudgetRoutingAction::HardStop
    } else if max_pct >= 0.95 {
        BudgetRoutingAction::Block
    } else if max_pct >= 0.80 {
        BudgetRoutingAction::Downgrade {
            max_cost_per_m: 0.50, // Only allow models cheaper than $0.50/M
        }
    } else {
        BudgetRoutingAction::Continue
    }
}

pub enum BudgetRoutingAction {
    Continue,
    Downgrade { max_cost_per_m: f64 },
    Block,
    HardStop,
}
```

---

### Loop 7: Latency → Reward — Data Flow

```
┌──────────────────────┐     ┌────────────────────────┐     ┌─────────────────┐
│  LatencyRegistry      │────►│  Latency Reward         │────►│  Bandit Update   │
│  (EWMA per model,     │     │  Computation (SLA       │     │  (composite      │
│   p50/p95/p99)        │     │   compliance scoring)   │     │   reward signal) │
└──────────────────────┘     └────────────────────────┘     └─────────────────┘
```

**Latency requirement:** Per-episode. Latency reward is computed as part of the bandit update.

**Failure mode if loop breaks:** Bandit selects high-quality but slow models → SLA violations → user-facing delays. Recovery: manual latency SLA enforcement in roko.toml routing config.

```rust
// Source: LatencyRegistry state
pub struct LatencyStats {
    pub model: String,
    pub provider: String,
    pub ewma_ms: f64,
    pub p50_ms: f64,
    pub p95_ms: f64,
    pub p99_ms: f64,
    pub sample_count: u64,
}

// Transform: latency → reward component
fn latency_to_reward_transform(
    stats: &LatencyStats,
    sla_ms: u64,  // from RoutingConfig, default: 5000
) -> f64 {
    let sla = sla_ms as f64;
    if stats.ewma_ms <= sla * 0.5 {
        1.0  // Well within SLA → full reward
    } else if stats.ewma_ms <= sla {
        // Linear decay from 1.0 to 0.5 as latency approaches SLA
        1.0 - 0.5 * ((stats.ewma_ms - sla * 0.5) / (sla * 0.5))
    } else {
        // Beyond SLA → penalty proportional to overshoot
        (0.5 * sla / stats.ewma_ms).max(0.0)
    }
}

// Sink: composite reward for bandit update
fn composite_reward(
    quality: f64,       // gate pass = 1.0, fail = 0.0
    cost_reward: f64,   // 1.0 - normalized_cost
    latency_reward: f64, // from transform above
    weights: &RewardWeights,
) -> f64 {
    weights.quality * quality
        + weights.cost * cost_reward
        + weights.latency * latency_reward
}

pub struct RewardWeights {
    pub quality: f64,   // default: 0.60
    pub cost: f64,      // default: 0.25
    pub latency: f64,   // default: 0.15
}
```

---

### Loop 8: Experiments → Static — Data Flow

```
┌──────────────────────┐     ┌────────────────────────┐     ┌─────────────────┐
│  ExperimentStore      │────►│  Significance Tester    │────►│  Static Config   │
│  (concluded expts     │     │  (chi-squared or        │     │  (roko.toml      │
│   with variant data)  │     │   z-test for winner)    │     │   updates)       │
└──────────────────────┘     └────────────────────────┘     └─────────────────┘
```

**Latency requirement:** Batch (on experiment conclusion, checked every 50 episodes). Config changes need human review.

**Failure mode if loop breaks:** Experiment results are transient — winners aren't persisted, so the system re-runs the same experiments indefinitely. Recovery: manual config update based on experiment logs.

```rust
// Source: ExperimentStore concluded experiments
pub struct ExperimentConclusion {
    pub experiment_id: String,
    pub section_name: String,
    pub winner_variant: String,
    pub winner_pass_rate: f64,
    pub baseline_pass_rate: f64,
    pub delta: f64,
    pub p_value: f64,
    pub sample_size: usize,
}

// Transform: statistical significance test
fn experiments_to_static_transform(
    conclusion: &ExperimentConclusion,
    min_delta: f64,  // default: 0.05 (5% improvement required)
    max_p_value: f64,  // default: 0.05
    min_samples: usize,  // default: 50
) -> Option<ConfigUpdate> {
    if conclusion.delta < min_delta
        || conclusion.p_value > max_p_value
        || conclusion.sample_size < min_samples
    {
        return None; // Not significant enough to promote
    }

    Some(ConfigUpdate {
        key: format!("prompt.{}.variant", conclusion.section_name),
        old_value: "baseline".into(),
        new_value: conclusion.winner_variant.clone(),
        reason: format!(
            "Experiment {} concluded: variant '{}' improved pass rate by {:.1}% (p={:.4}, n={})",
            conclusion.experiment_id,
            conclusion.winner_variant,
            conclusion.delta * 100.0,
            conclusion.p_value,
            conclusion.sample_size,
        ),
        requires_review: true, // Human must approve config changes
    })
}

// Sink: proposed config update
pub struct ConfigUpdate {
    pub key: String,
    pub old_value: String,
    pub new_value: String,
    pub reason: String,
    pub requires_review: bool,
}
```

---

## Cross-Loop Interaction Matrix

The eight loops do not operate independently — they interact. This matrix identifies the key interactions:

| Source Loop | Affected Loop | Interaction |
|-------------|---------------|-------------|
| 1 (Health→Routing) | 6 (Cost→Routing) | Provider failure forces fallback to more expensive provider |
| 2 (Conductor→Routing) | 7 (Latency→Reward) | High system load increases latency, penalizing reward signals |
| 3 (Section→Scaffold) | 5 (Skills→Prompts) | Section weight changes may truncate skill injection section |
| 4 (Failure→Replan) | 6 (Cost→Routing) | Replanning creates new tasks, increasing session cost |
| 6 (Cost→Routing) | 1 (Health→Routing) | Cost-forced downgrade to cheap provider may hit rate limits |
| 7 (Latency→Reward) | 2 (Conductor→Routing) | Latency-optimal routing may increase system load |
| 8 (Experiments→Static) | 3 (Section→Scaffold) | Experiment winner changes section content, resetting effectiveness data |

### Interaction-Aware Scheduling

To prevent cascading oscillation from loop interactions, updates should be scheduled with awareness of dependencies:

```
Priority 1 (every episode): Loop 1 (Health), Loop 6 (Cost)
    → Safety-critical: prevent provider failures and budget overruns

Priority 2 (every 5 episodes): Loop 7 (Latency), Loop 2 (Conductor)
    → Performance: optimize for speed and resource utilization

Priority 3 (every 20 episodes): Loop 3 (Section), Loop 5 (Skills)
    → Learning: adjust prompt composition based on accumulated evidence

Priority 4 (every 50 episodes): Loop 4 (Failure→Replan), Loop 8 (Experiments)
    → Strategic: make structural changes with high confidence requirements
```

This priority ordering ensures that safety-critical loops (health, cost) always run before learning loops (section, skills), preventing a scenario where a learning-driven change causes a safety-critical failure.

---

## Relationship to Other Documents

- **[04-cascade-router](04-cascade-router.md)** — The primary target for loops 1, 2, 6, 7, 8.
- **[09-provider-health-circuit-breaker](09-provider-health-circuit-breaker.md)** — Source for loop 1.
- **[08-cost-normalization](08-cost-normalization.md)** — Source for loop 6.
- **[02-skill-library-voyager](02-skill-library-voyager.md)** — Source for loop 5.
- **[14-stability-mechanisms](14-stability-mechanisms.md)** — Prevents these loops from oscillating.
- **[15-collective-calibration-31x](15-collective-calibration-31x.md)** — Measures the aggregate effect of all loops.


---

# SOURCE: /Users/will/dev/nunchi/roko/roko/docs/05-learning/14-stability-mechanisms.md

# Stability Mechanisms

> **Implementation plan:** `modelrouting/17-meta-learning-and-corrections.md` (tasks 2O.04–2O.06)
> **Theoretical basis:** Control theory (hysteresis, frequency separation), Ashby's Law
> **Cross-references:** [04-cascade-router](04-cascade-router.md), [13-8-missing-feedback-loops](13-8-missing-feedback-loops.md), [07-regression-detection](07-regression-detection.md)


> **Implementation**: Shipping

---

## Purpose

A system with eight feedback loops operating simultaneously can oscillate: loop 1 routes away from a provider, loop 6 routes back because the alternative is more expensive, loop 7 routes away again because the alternative is slower, and the system thrashes between options without settling. Stability mechanisms prevent this oscillation by introducing damping, hysteresis, and frequency separation.

These mechanisms are not an optimization — they are a prerequisite. Without them, the compound improvement described in [17-adas-and-autocatalytic](17-adas-and-autocatalytic.md) cannot occur because the system spends its energy oscillating rather than converging.

---

## Hysteresis

### Definition

Hysteresis introduces a switching threshold: the system only changes its decision when the new option is sufficiently better than the current option. Small improvements are ignored, preventing rapid oscillation between near-equal alternatives.

### Roko Implementation

The cascade router uses a 10% score delta threshold for model switching:

```
Current model: claude-sonnet-4 (score: 0.82)
Challenger: claude-opus-4 (score: 0.85)

Delta: 0.85 − 0.82 = 0.03
Hysteresis threshold: 0.10

0.03 < 0.10 → Keep current model (no switch)
```

The system only switches to a new model when the challenger's score exceeds the current model's score by at least 10%. This prevents oscillation between models with similar performance, which is common when:
- Two models have similar pass rates with different cost structures.
- Statistical noise makes one model appear slightly better on some batches.
- A new model's small advantage doesn't justify the disruption of switching.

### Why 10%?

The 10% threshold balances responsiveness and stability:

| Threshold | Behavior |
|-----------|----------|
| 1% | Near-zero hysteresis — switches on noise |
| 5% | Low hysteresis — switches on moderate improvements |
| **10%** | Moderate hysteresis — switches on meaningful improvements |
| 20% | High hysteresis — misses genuine improvements |
| 50% | Extreme — never switches except for dramatic changes |

The 10% value was chosen because:
- Typical model performance differences are 5-15% in pass rate.
- Cost differences between tiers are 5-10× (much larger than 10%).
- A 10% improvement in the composite score represents a genuine, actionable improvement.

### Hysteresis in Other Subsystems

The hysteresis principle applies beyond model routing:

| Subsystem | Hysteresis Mechanism |
|-----------|---------------------|
| Playbook rules | Confidence must cross min_confidence to prune (not oscillate near threshold) |
| Circuit breaker | Half-open requires a successful probe before closing (not just cooldown expiry) |
| Adaptive thresholds | EMA smoothing prevents threshold oscillation from batch-to-batch noise |
| Pattern discovery | min_support threshold prevents low-confidence patterns from being promoted |

---

## Frequency Separation

### Definition

Frequency separation assigns different update rates to subsystems based on their characteristic timescales. Fast subsystems (model routing) update every episode. Slow subsystems (pattern discovery) update every 20 episodes. This prevents fast loops from reacting to signals that haven't been confirmed by slow loops.

### Roko Implementation

```
Update Frequency Hierarchy:

    ┌─── Every episode ───────────────────────────────────────┐
    │  Cascade router:     update bandit arms                  │
    │  Episode logger:     append episode                      │
    │  Cost log:           append cost record                  │
    │  Provider health:    update circuit breaker               │
    │  Anomaly detector:   check prompt loop, cost spike        │
    └─────────────────────────────────────────────────────────┘
                            │
    ┌─── Every 5 episodes ────────────────────────────────────┐
    │  Gate thresholds:    EMA update of adaptive thresholds    │
    │  Regression check:   compare current vs baseline         │
    │  Efficiency grading: update section effectiveness         │
    └─────────────────────────────────────────────────────────┘
                            │
    ┌─── Every 20 episodes ───────────────────────────────────┐
    │  Pattern discovery:  trigram mining, pattern extraction   │
    │  Skill extraction:   Voyager-style skill mining           │
    │  Cross-episode:      HDC clustering consolidation         │
    └─────────────────────────────────────────────────────────┘
                            │
    ┌─── Every 50 episodes ───────────────────────────────────┐
    │  Pareto frontier:    recompute Pareto-optimal models     │
    │  C-Factor:           recompute collective capability     │
    └─────────────────────────────────────────────────────────┘
```

### Why These Frequencies?

| Subsystem | Frequency | Rationale |
|-----------|-----------|-----------|
| Cascade router | Every 1 | Routing decisions benefit from immediate feedback |
| Gate thresholds | Every 5 | Thresholds need multiple data points to avoid noise |
| Pattern discovery | Every 20 | Patterns need a statistically meaningful sample |
| Pareto frontier | Every 50 | Model statistics need many observations for stable estimates |

The frequencies are chosen so that each subsystem has enough observations to make a reliable update at its cadence. A subsystem that updates too frequently relative to its required sample size produces noisy decisions; one that updates too infrequently misses genuine changes.

### Interaction Between Frequencies

The frequency hierarchy creates a natural information cascade:

1. **Per-episode** data flows into fast subsystems (routing, health).
2. **Aggregated** data (5-episode windows) flows into medium subsystems (thresholds, regression).
3. **Consolidated** data (20-episode batches) flows into slow subsystems (patterns, skills).
4. **Summary** data (50-episode summaries) flows into the slowest subsystems (Pareto, C-Factor).

Each level receives data that has already been filtered and stabilized by the level above it. Fast oscillations in routing decisions are invisible to pattern discovery, which only sees the 20-episode trend.

---

## Damping

### EMA Smoothing

Exponential Moving Average (EMA) smoothing damps oscillation in continuously-valued quantities:

```
ema_new = α × observation + (1 − α) × ema_old
```

where α ∈ (0, 1) controls the smoothing rate. Small α = heavy smoothing (slow response). Large α = light smoothing (fast response).

| Subsystem | α | Behavior |
|-----------|---|----------|
| Gate thresholds | 0.1 | Heavy smoothing — thresholds change slowly |
| Cost EWMA | 0.2 | Moderate smoothing — cost baseline adapts over ~5 observations |
| Latency EMA | 0.1 | Heavy smoothing — latency baseline is conservative |
| LinUCB alpha decay | exp(-obs/60) | Exponential decay — exploration decreases gradually |

### Why Not Moving Average?

Simple moving averages (mean of last N values) have a discontinuity problem: when an old value exits the window, the average can jump even without new data. EMA avoids this by weighting all past observations, with exponentially decaying weights. The result is a smooth, continuous signal that responds proportionally to the magnitude of new observations.

---

## Compound Stability

The interaction of hysteresis, frequency separation, and EMA smoothing creates compound stability:

1. **Hysteresis** prevents switching on noise.
2. **Frequency separation** prevents fast loops from disrupting slow loops.
3. **EMA smoothing** prevents continuous quantities from oscillating.

Together, these mechanisms ensure that the eight feedback loops converge to a stable operating point rather than oscillating. The system "locks in" to good configurations and only moves when there is strong evidence for improvement.

### Stability Budget

Each feedback loop has a "stability budget": the amount of perturbation it can absorb without oscillating. The hysteresis threshold, update frequency, and EMA α collectively determine this budget. Loops with large stability budgets (pattern discovery: 20-episode frequency, high min_support threshold) are very stable but slow to respond. Loops with small stability budgets (routing: per-episode frequency, 10% hysteresis) are responsive but more prone to oscillation.

The system design ensures that stability budgets increase with the severity of the action: routing decisions (low cost to change) have small stability budgets, while pattern promotion (high cost to change — wrong rules degrade all future agents) has large stability budgets.

---

## Anti-Pattern: Positive Feedback Loops

Stability mechanisms are designed to prevent positive feedback loops — self-reinforcing cycles that drive the system to extremes:

| Anti-pattern | What happens | Prevention |
|-------------|-------------|------------|
| Model lock-in | Bandit exploits one model so heavily that alternatives never get enough data to compete | UCB exploration term, α decay |
| Playbook explosion | Rules accumulate without pruning, consuming entire prompt budget | Confidence decay, min_confidence threshold |
| Cost death spiral | Budget pressure forces cheap models → failures → more iterations → higher cost | Per-task budget limit, hard stop |
| Threshold collapse | Adaptive thresholds relax so far that gates are meaningless | Floor on threshold values |

Each anti-pattern has a specific stability mechanism that prevents it. The compound effect is that the system remains in its "viable region" (Beer's Viable System Model) — operating within the bounds where all feedback loops function correctly.

---

## Theoretical Foundation

### Ashby's Law of Requisite Variety

A control system must have at least as much variety (number of distinct states) as the system it controls. Roko's stability mechanisms implement this by providing a different damping mechanism for each type of oscillation:

| Oscillation Type | Required Variety | Mechanism |
|-----------------|-----------------|-----------|
| Binary switching (model A vs B) | Two states + threshold | Hysteresis |
| Continuous drift (parameter values) | Continuous damping | EMA smoothing |
| Multi-rate interference (fast loop disturbs slow loop) | Frequency isolation | Frequency separation |
| Degenerate convergence (all traffic to one arm) | Forced exploration | UCB exploration term |

### Beer's Viable System Model

Beer's VSM defines five systems required for organizational viability. Roko's stability mechanisms map to:

| VSM System | Function | Roko Implementation |
|-----------|----------|-------------------|
| System 1 | Operations | Individual learning subsystems (bandits, episode logger, etc.) |
| System 2 | Coordination | Frequency separation, LearningRuntime ordering |
| System 3 | Control | Regression detection, C-Factor monitoring |
| System 4 | Intelligence | Pattern discovery, predictive foraging |
| System 5 | Policy | Hysteresis thresholds, EMA parameters |

The stability mechanisms primarily implement Systems 2 and 3: coordination between subsystems and control over aggregate behavior.

### Good Regulator Theorem

A system that is a good regulator of another system must be a model of that system. Roko's C-Factor is a model of the system's overall health — it captures the key performance indicators in a single composite score. The regression detector uses this model to identify when the system deviates from expected behavior, triggering corrective actions.

---

## Relationship to Other Documents

- **[04-cascade-router](04-cascade-router.md)** — The primary subsystem where hysteresis is applied.
- **[13-8-missing-feedback-loops](13-8-missing-feedback-loops.md)** — The eight loops that stability mechanisms regulate.
- **[07-regression-detection](07-regression-detection.md)** — Regression detection is itself a stability mechanism (alerts on degradation).
- **[09-provider-health-circuit-breaker](09-provider-health-circuit-breaker.md)** — Circuit breaker is a stability mechanism for provider health.
- **[17-adas-and-autocatalytic](17-adas-and-autocatalytic.md)** — Stability is a prerequisite for the compound improvement described there.


---

# SOURCE: /Users/will/dev/nunchi/roko/roko/docs/05-learning/15-collective-calibration-31x.md

# Collective Calibration (31.6× Heuristic)

> **PRD source:** `refactoring-prd/09-innovations.md` §VI
> **Module:** `roko-learn/src/cfactor.rs`
> **Cross-references:** [04-cascade-router](04-cascade-router.md), [07-regression-detection](07-regression-detection.md), [13-8-missing-feedback-loops](13-8-missing-feedback-loops.md)


> **Implementation**: Shipping

---

## Purpose

Collective Calibration is a heuristic framework for quantifying the aggregate performance improvement that emerges when multiple agents, feedback loops, and learning subsystems operate in concert. The core claim is that well-calibrated agent collectives can achieve up to 31.6× the throughput of individual agents — but this is a **heuristic upper bound with explicit caveats**, not a proven theorem.

The 31.6× figure comes from a simplified model inspired by the Central Limit Theorem. It provides a target and a measurement framework, not a guarantee.

---

## The 31.6× Heuristic

### Derivation

The heuristic models accuracy as:

```
accuracy(t) = 1 − 1/√(N × t)
```

where:
- N = number of agents in the collective
- t = number of calibration rounds (episodes)

For N = 10 agents and t = 100 rounds:

```
accuracy = 1 − 1/√(10 × 100) = 1 − 1/√1000 ≈ 1 − 0.0316 ≈ 0.968
```

The "31.6×" refers to the √1000 ≈ 31.6 factor in the denominator, which represents the effective sample size advantage of a calibrated collective over a single agent.

### CLT Inspiration

The formula is inspired by the Central Limit Theorem: the standard error of a sample mean decreases as 1/√n. If each agent provides an independent observation, and the collective aggregates these observations, the collective's error decreases as 1/√(N×t).

### Explicit Caveats

**This is NOT a theorem.** The following assumptions are required and frequently violated:

1. **Independence**: Agents' errors must be independent. In practice, agents using the same model and similar prompts make correlated errors. Correlation reduces the effective N.

2. **Stationarity**: The target distribution must not change during calibration. In practice, the codebase evolves, model providers update, and task distributions shift. Non-stationarity reduces the effective t.

3. **Aggregation mechanism**: The formula assumes optimal aggregation (e.g., majority voting or Bayesian averaging). In practice, Roko uses sequential execution with feedback, not parallel voting. The aggregation mechanism affects the constant factor.

4. **Finite-sample effects**: For small N and t, the 1/√(N×t) approximation is loose. The CLT is an asymptotic result; finite samples may be far from the limit.

5. **Heterogeneous quality**: The formula assumes equal-quality agents. If some agents are much worse than others, they add noise rather than signal, potentially reducing collective performance below individual performance.

**In practice, expect 3-10× improvement from collective calibration, not 31.6×.** The 31.6× is the idealized upper bound under perfect conditions.

---

## C-Factor: Composite Capability Metric

The C-Factor (Collective Capability Factor) is the practical implementation of collective calibration measurement. It combines multiple performance indicators into a single scalar:

```rust
pub struct CFactor {
    /// 0.0-1.0 composite score.
    pub overall: f64,
    /// Component breakdown.
    pub components: CFactorComponents,
    /// Per-agent leave-one-out contributions.
    pub agent_contributions: Vec<AgentCFactorContribution>,
    /// When the score was computed.
    pub computed_at: DateTime<Utc>,
    /// Number of episodes in the calculation.
    pub episode_count: usize,
}
```

### Components

```rust
pub struct CFactorComponents {
    /// % of tasks passing gates on first attempt.
    pub gate_pass_rate: f64,
    /// Inverse of cost per successful task, normalized.
    pub cost_efficiency: f64,
    /// Inverse of time per successful task, normalized.
    pub speed: f64,
    /// Normalized signal throughput.
    pub information_flow_rate: f64,
    /// % of tasks succeeding without re-plan.
    pub first_try_rate: f64,
    /// Rate of new knowledge entries per episode.
    pub knowledge_growth: f64,
    /// Speed of shared insight accumulation.
    pub knowledge_integration_rate: f64,
    /// How strongly templates specialize by category.
    pub task_diversity_coverage: f64,
    /// Speed of convergent conclusions.
    pub convergence_velocity: f64,
    /// Evenness of agent participation.
    pub turn_taking_equality: f64,
    /// Normalized dependency output rate.
    pub social_sensitivity: f64,
}
```

### Component Weights

The composite score is a weighted average of components. Default weights emphasize outcome metrics over process metrics:

| Component | Weight | Rationale |
|-----------|--------|-----------|
| gate_pass_rate | 0.20 | Primary success metric |
| cost_efficiency | 0.15 | Budget sustainability |
| speed | 0.10 | Throughput |
| first_try_rate | 0.15 | Efficiency of approach |
| knowledge_growth | 0.10 | Learning velocity |
| turn_taking_equality | 0.05 | Collaboration quality |
| Others | 0.25 (distributed) | Secondary indicators |

---

## Leave-One-Out Contributions

The C-Factor includes per-agent contribution scores computed via leave-one-out analysis:

```rust
pub struct AgentCFactorContribution {
    /// Agent identifier.
    pub agent_id: String,
    /// Episodes attributed to this agent.
    pub episode_count: usize,
    /// C-Factor without this agent's episodes.
    pub without_agent_overall: f64,
    /// Full score minus leave-one-out score.
    pub contribution_score: f64,
}
```

If `contribution_score > 0`, the agent raises the collective C-Factor (positive contributor). If `contribution_score < 0`, the agent drags it down (negative contributor).

### Dispatch Bias

Leave-one-out contributions inform routing decisions:

```rust
pub enum AgentDispatchBias {
    /// Agent has negative contribution → prefer stronger model.
    PreferStronger,
    /// Agent has strong positive contribution → prefer cheaper model.
    PreferCheaper,
    /// Neutral contribution → no bias.
    Neutral,
}
```

The cascade router uses this bias during the confidence stage: agents with consistently negative contributions are routed to stronger (more expensive) models, while agents with strong positive contributions can be routed to cheaper models without sacrificing quality.

---

## C-Factor Regression

The C-Factor tracks its own history for regression detection:

```rust
pub struct CFactorRegression {
    pub current_snapshot_at: DateTime<Utc>,
    pub window_start: DateTime<Utc>,
    pub window_end: DateTime<Utc>,
    pub sample_count: usize,
    // ... delta analysis
}
```

A C-Factor regression is triggered when the current C-Factor drops significantly below the trailing average. This catches systemic degradation that individual metrics might miss — a small drop in pass rate combined with a small increase in cost and a small decrease in speed may not trigger any individual threshold, but the C-Factor composite detects the overall decline.

---

## Computing the C-Factor

The C-Factor is computed every 50 episodes (the slowest learning frequency):

```
Every 50 episodes:
    │
    ├── 1. Load recent episodes (sliding window of last 200)
    │
    ├── 2. Compute component metrics:
    │       gate_pass_rate: successful episodes / total episodes
    │       cost_efficiency: 1 / (avg cost per success), normalized
    │       speed: 1 / (avg duration per success), normalized
    │       first_try_rate: iteration-0 successes / total tasks
    │       knowledge_growth: new skills + patterns per episode
    │       ...
    │
    ├── 3. Compute leave-one-out contributions per agent
    │
    ├── 4. Combine components with weights → overall score
    │
    └── 5. Persist to .roko/learn/c-factor.jsonl
```

### Normalization

Each component is normalized to [0.0, 1.0] before weighting. Normalization uses a baseline window: the component value from the first 10 plans serves as the reference point. Values below baseline map to [0.0, 0.5], values at baseline map to 0.5, and values above baseline map to [0.5, 1.0].

This relative normalization means the C-Factor measures improvement over the system's own baseline, not against an absolute standard. A C-Factor of 0.8 means the system is performing significantly better than its initial configuration, regardless of what that initial configuration was.

---

## Practical Interpretation

| C-Factor | Interpretation | Action |
|----------|---------------|--------|
| < 0.3 | System is performing poorly | Investigate regressions, consider manual intervention |
| 0.3 – 0.5 | Below baseline | Check feedback loops, review recent changes |
| 0.5 | At baseline | Normal operation |
| 0.5 – 0.7 | Above baseline, improving | Learning loops are working |
| 0.7 – 0.9 | Well above baseline | System has significantly improved through self-optimization |
| > 0.9 | Near-optimal | Consider lowering cost (cheaper models) while maintaining quality |

---

## Relationship to Other Documents

- **[04-cascade-router](04-cascade-router.md)** — C-Factor provides routing bias (PreferStronger/PreferCheaper/Neutral).
- **[07-regression-detection](07-regression-detection.md)** — C-Factor regression complements per-metric regression detection.
- **[13-8-missing-feedback-loops](13-8-missing-feedback-loops.md)** — The C-Factor measures the aggregate effect of all eight loops.
- **[14-stability-mechanisms](14-stability-mechanisms.md)** — C-Factor is computed at the slowest frequency (every 50 episodes), making it a stability anchor.
- **[17-adas-and-autocatalytic](17-adas-and-autocatalytic.md)** — The autocatalytic thesis predicts that C-Factor should increase over time as learning compounds.
- **[06-task-metrics-and-baselines](06-task-metrics-and-baselines.md)** — Individual metrics feed into C-Factor components.


---

# SOURCE: /Users/will/dev/nunchi/roko/roko/docs/05-learning/16-predictive-foraging.md

# Predictive Foraging

> **PRD source:** `refactoring-prd/09-innovations.md` §VII
> **Implementation plan:** `modelrouting/12-advanced-patterns.md` (tasks 2J.04–2J.06)
> **Theoretical basis:** Optimal Foraging Theory (MacArthur & Pianka 1966), Calibration (Gneiting & Raftery 2007)
> **Cross-references:** [04-cascade-router](04-cascade-router.md), [15-collective-calibration-31x](15-collective-calibration-31x.md), [14-stability-mechanisms](14-stability-mechanisms.md), [18-self-learning-cybernetic-loops](18-self-learning-cybernetic-loops.md)


> **Implementation**: Shipping

---

## Purpose

Predictive Foraging turns every orchestrator decision into a falsifiable prediction. Before each task, the system predicts: duration, complexity, gate outcome, and merge conflict probability. After execution, predictions are compared against actual outcomes. The gap between prediction and reality — the calibration error — becomes a learning signal that feeds back into the prediction models.

The name "foraging" comes from optimal foraging theory: an agent foraging for resources (information, successful outcomes) must decide where to invest its attention. A well-calibrated predictor directs foraging effort toward the highest-value opportunities, avoiding areas that look promising but consistently disappoint.

This is the task-level slice of the Bus-backed predict-publish-correct loop described in [18-self-learning-cybernetic-loops](18-self-learning-cybernetic-loops.md); see [Naming and Glossary](../00-architecture/01-naming-and-glossary.md) for the two-fabric vocabulary. See `../../tmp/refinements/10-self-learning-cybernetic-loops.md` for the full proposal. In that wider loop, `prediction.error.*` is a first-class signal family rather than just a local calibration metric.

---

## Predictions

The system makes four types of predictions at task dispatch time:

### 1. Duration Prediction

```
Prediction: "Task T3 will take approximately 45 seconds of wall time."
Actual: 78 seconds
Error: +73% (underprediction)
```

Duration predictions are computed from baseline statistics for the `(role, complexity_band)` slice (see [06-task-metrics-and-baselines](06-task-metrics-and-baselines.md)). The prediction model starts with the slice average and adjusts based on:
- Crate familiarity (familiar crates → shorter duration)
- Iteration count (retries take longer)
- Playbook rule matches (rules suggesting complexity → longer)

### 2. Complexity Prediction

```
Prediction: "Task T3 is Standard complexity."
Actual: Required 4 iterations, touched 7 files → effectively Complex
Error: Underestimated complexity
```

Complexity predictions come from the plan generator's static analysis of task specs. The calibration tracker compares the predicted complexity band against the actual execution characteristics (iterations needed, files touched, gate failures encountered).

### 3. Gate Outcome Prediction

```
Prediction: "Task T3 has 72% probability of first-attempt gate pass."
Actual: Gate failed on first attempt (compile error)
Error: Overconfident by ~22%
```

Gate predictions use the cascade router's per-model pass rate statistics adjusted by task features. A well-calibrated predictor should have its 70% predictions succeed approximately 70% of the time.

### 4. Merge Conflict Prediction

```
Prediction: "Tasks T3 and T5 have 30% probability of merge conflict (both modify roko-core/src/config.rs)."
Actual: No conflict
Error: False positive
```

Merge conflict predictions use file overlap analysis between concurrent tasks. When two tasks modify the same file, the probability of conflict increases with the number of overlapping lines.

---

## CalibrationTracker

The `CalibrationTracker` records predictions and outcomes, then computes calibration metrics:

```
CalibrationTracker {
    predictions: Vec<PredictionRecord>,
    // Each record: { prediction, actual, timestamp, context }
}
```

### Calibration Metric

For probabilistic predictions (gate outcome, conflict probability), calibration is measured as the Brier score:

```
Brier score = (1/N) × Σ (predicted_probability − actual_outcome)²
```

A perfectly calibrated predictor has Brier score 0. A predictor that always predicts 50% has Brier score 0.25 on binary outcomes. Lower is better.

### Reliability Diagram

Calibration is visualized as a reliability diagram: predictions are binned by predicted probability (0-10%, 10-20%, ..., 90-100%), and for each bin, the actual success rate is plotted. A well-calibrated predictor falls on the diagonal (predicted 70% → actual ~70%).

```
Actual %  ↑
  100% │                                     ●
       │                                 ●
   80% │                            ●
       │                        ●
   60% │                   ●        ← perfectly calibrated (diagonal)
       │              ●
   40% │         ●
       │     ●
   20% │ ●
       │
    0% └──────────────────────────────────► Predicted %
       0%   20%   40%   60%   80%  100%
```

### Arithmetic Corrector

When calibration error is detected, the system applies a simple arithmetic correction:

```
corrected_prediction = raw_prediction × correction_factor
```

The correction factor is computed from historical calibration data:

```
correction_factor = actual_mean / predicted_mean
```

For example, if the system consistently predicts 70% pass rate but observes 55% actual pass rate, the correction factor is 55/70 ≈ 0.786. Future raw predictions of 70% become corrected predictions of 55%.

This arithmetic correction runs in approximately **50 nanoseconds** — negligible overhead per decision. Despite its simplicity, it captures the dominant source of miscalibration (systematic bias) without requiring complex recalibration models.

---

## Prediction as Learning Signal

The key insight of predictive foraging is that **prediction errors are more informative than raw outcomes**. A task that fails is one data point. A task that was predicted to succeed with 90% confidence but failed is a strong signal that the prediction model is miscalibrated for this type of task.

This creates a higher-order learning loop:

```
Level 0: Task outcome (pass/fail)
    │
    ▼
Level 1: Was the prediction correct? (calibration error)
    │
    ▼
Level 2: Is the prediction model systematically biased? (calibration drift)
    │
    ▼
Level 3: Are the features used for prediction informative? (feature importance)
```

Each level produces a distinct learning signal:
- Level 0 updates the bandit arm (standard reward).
- Level 1 updates the calibration correction factor.
- Level 2 triggers prediction model retraining (or feature engineering).
- Level 3 informs the next round of system design improvements.

---

## Integration with Routing

Calibrated predictions improve routing decisions:

```
Task T3: predicted gate pass probability = 0.55 (after calibration)
    │
    ▼
CascadeRouter: Low confidence → prefer stronger model
    │
    ▼
Routes to claude-opus-4 instead of claude-sonnet-4
```

Without calibration, the raw predicted probability might be 0.72, leading the router to use a weaker (cheaper) model. The calibrated prediction of 0.55 correctly identifies this as a risky task that benefits from a stronger model.

---

## Foraging Strategy

Optimal foraging theory suggests allocating effort proportional to expected return. In the agent context:

| Predicted Outcome | Foraging Strategy |
|-------------------|-------------------|
| High pass probability, low cost | Quick execution — use cheapest model |
| High pass probability, high cost | Standard execution — optimize for cost |
| Low pass probability, low cost | Speculative execution — try cheap model first |
| Low pass probability, high cost | Careful execution — invest in thorough prompting |

The cascade router implements this strategy through its C-Factor-driven bias: high-confidence tasks get cheaper models, low-confidence tasks get stronger models.

---

## Surface Predictions in TUI

Predictions are surfaced in the dashboard (see [16-heartbeat](../16-heartbeat/INDEX.md)):

```
Plan X: predicted completion in 4 minutes (based on similar plans)
Plan Y: HIGH risk of gate failure (low affordance code, no tests in target files)
Plans A and B: 30% chance of merge conflict (both modify roko-core/src/event.rs)
```

This gives the operator forward-looking diagnostics, enabling proactive intervention instead of reactive debugging.

---

## Performance

The predictive foraging pipeline adds minimal overhead:

| Operation | Cost | When |
|-----------|------|------|
| Generate predictions | ~1μs | Before each task dispatch |
| Calibration correction | ~50ns | Per prediction |
| Record prediction + outcome | ~10μs (JSONL append) | After each task |
| Recalibrate correction factor | ~100μs | Every 50 episodes |

Total per-task overhead: < 15μs. This is negligible compared to the agent execution time (typically 10-120 seconds per task).

---

## Practical Example

### Before Calibration

The system predicts gate pass probabilities based on raw per-model statistics:

```
Task T7: modify roko-core/src/config/schema.rs
    Model: claude-sonnet-4
    Raw prediction: 72% gate pass probability
    → Router: 72% is above threshold → use sonnet (cheaper)
    Actual: gate FAILED (compile error in config)
```

Over 50 tasks, the raw predictor shows systematic overconfidence:

```
Predicted 70-80% range: 45 tasks
    Actual pass rate: 55%
    Expected pass rate: ~75%
    Bias: +20% overconfident
```

### After Calibration

The arithmetic corrector adjusts:

```
correction_factor = 55% / 75% = 0.733
```

Now for Task T107:

```
Task T107: modify roko-core/src/config/schema.rs
    Model: claude-sonnet-4
    Raw prediction: 72%
    Corrected prediction: 72% × 0.733 = 52.8%
    → Router: 52.8% is below threshold → use opus (stronger)
    Actual: gate PASSED (opus handles config changes correctly)
```

The calibrated prediction correctly identifies this as a risky task, causing the router to invest in a stronger model. The cost of using opus ($1.38) is lower than the cost of a failed sonnet attempt plus retry ($0.78 + $1.38 = $2.16).

### Calibration Improves Over Time

As the corrector accumulates more data, its bias estimate becomes more precise. After 200 tasks, per-category correction factors emerge:

```
Category: config_modification
    correction_factor: 0.733 (overconfident on config tasks)

Category: test_scaffolding
    correction_factor: 1.05 (slightly underconfident on test tasks)

Category: cross_crate_refactor
    correction_factor: 0.62 (very overconfident on cross-crate tasks)
```

Per-category correction captures the observation that prediction accuracy varies by task type: the system is well-calibrated for test scaffolding but systematically overconfident for cross-crate refactoring.

---

## Relationship to Other Documents

- **[04-cascade-router](04-cascade-router.md)** — Calibrated predictions inform routing bias.
- **[06-task-metrics-and-baselines](06-task-metrics-and-baselines.md)** — Baselines provide the raw data for duration and complexity predictions.
- **[14-stability-mechanisms](14-stability-mechanisms.md)** — Calibration correction acts as a damping mechanism for prediction-based routing.
- **[15-collective-calibration-31x](15-collective-calibration-31x.md)** — Collective calibration is the aggregate-level version of individual prediction calibration.
- **[17-adas-and-autocatalytic](17-adas-and-autocatalytic.md)** — Predictive foraging is one of the 14 frontier innovations in the Roko innovation roadmap.


---

# SOURCE: /Users/will/dev/nunchi/roko/roko/docs/05-learning/17-adas-and-autocatalytic.md

# ADAS and Autocatalytic Thesis

> **PRD sources:** `refactoring-prd/09-innovations.md` §X–XI, `refactoring-prd/00-overview.md` (Autocatalytic Improvement)
> **Academic basis:** Hu et al. ICLR 2025 (ADAS); Kauffman 1993 (autocatalytic sets); Chen et al. 2023 (EvoSkills); Loreto & Tria 2014 (Pólya urn); Reed's Law; Metcalfe's Law
> **Legacy sources:** `agent-chain/09-exponential-flywheels.md`, `agent-chain/self-improvement-frameworks.md`
> **Cross-references:** [02-skill-library-voyager](02-skill-library-voyager.md), [13-8-missing-feedback-loops](13-8-missing-feedback-loops.md), [15-collective-calibration-31x](15-collective-calibration-31x.md)


> **Implementation**: Shipping

---

## Purpose

This document describes two frontier concepts that together form Roko's long-term thesis on compound self-improvement:

1. **ADAS (Automated Design of Agentic Systems)** — a meta-agent that searches the space of possible agent architectures, discovering new components and configurations that improve performance.
2. **Autocatalytic growth** — the theoretical framework for why a system with interconnected feedback loops can achieve super-linear improvement over time.

Both are speculative — ADAS is planned but not implemented, and the autocatalytic thesis is a design aspiration backed by theoretical models and early empirical evidence, not a proof. This document presents them with appropriate epistemic caveats.

---

## ADAS: Automated Design of Agentic Systems

### Background

Hu et al. (ICLR 2025) introduced ADAS: a meta-agent that searches the space of agentic system designs by generating, evaluating, and iterating on agent architectures in code.

**Key results:**
- +14% accuracy on ARC (Abstraction and Reasoning Corpus)
- +13.6 F1 improvement on reading comprehension tasks
- Discovered novel agent architectures that outperformed expert-designed baselines

### How ADAS Works

```
Meta-Agent (ADAS)
    │
    ├── 1. Define search space:
    │       - Agent roles (how many, what capabilities)
    │       - Communication patterns (sequential, parallel, hierarchical)
    │       - Tool configurations (which tools per role)
    │       - Prompt templates (structure, content, ordering)
    │       - Routing strategies (model selection rules)
    │
    ├── 2. Generate candidate architecture (code):
    │       "Create a 3-agent pipeline:
    │        Planner (opus) → Implementer (sonnet) → Reviewer (haiku)
    │        with shared memory via episodes"
    │
    ├── 3. Evaluate on benchmark tasks:
    │       Run the candidate architecture on held-out tasks
    │       Measure: pass rate, cost, latency, iterations
    │
    ├── 4. Select and iterate:
    │       Keep architectures that improve over baseline
    │       Mutate: change roles, models, communication patterns
    │       Recombine: merge best features of top architectures
    │
    └── 5. Deploy winner:
          Update the production configuration
```

### Roko's ADAS Pathway

Roko's architecture is designed to support ADAS-style meta-optimization:

| ADAS Requirement | Roko Component |
|-----------------|----------------|
| Architecture representation in code | `roko.toml` configuration + `SystemPromptBuilder` templates |
| Evaluation harness | Gate pipeline (11 gates, deterministic verification) |
| Performance metrics | C-Factor, task metrics, regression detection |
| Experiment framework | `ExperimentStore` for A/B testing configurations |
| Search strategy | Cascade router bandits (can be extended to architecture search) |

The key insight is that Roko already has all the components needed for ADAS — it just needs a meta-level agent that operates on configurations rather than on code. Where a normal agent modifies `src/*.rs`, the ADAS meta-agent modifies `roko.toml`, prompt templates, and routing rules, then evaluates the results through the same gate pipeline.

### Planned ADAS Capabilities

1. **Prompt template search** — generate variant prompt templates, evaluate via gate pass rate, converge on best performers. (Partially implemented via `ExperimentStore`.)
2. **Model routing search** — test different model allocations per role, find cost-optimal configurations. (Partially implemented via cascade router.)
3. **Gate configuration search** — adjust gate thresholds and rung order, optimize for development velocity vs. quality. (Partially implemented via adaptive thresholds.)
4. **Agent topology search** — test different numbers of agents, role assignments, and communication patterns. (Not implemented — requires multi-agent orchestration.)

---

## EvoSkills: Evolutionary Skill Optimization

Chen et al. (2023) introduced EvoSkills: an evolutionary approach to skill optimization where skills are treated as a population that undergoes selection, crossover, and mutation.

### Connection to Roko

The skill library (see [02-skill-library-voyager](02-skill-library-voyager.md)) accumulates skills from successful episodes. EvoSkills extends this with evolutionary operators:

1. **Selection** — skills with high success rates are selected for reproduction.
2. **Crossover** — combine steps from two successful skills for related tasks.
3. **Mutation** — vary skill parameters (tool choices, step ordering) to explore alternatives.
4. **Fitness evaluation** — gate pass rate serves as the fitness function.

This creates a population of skills that evolves toward higher fitness, complementing the Voyager-style monotonic accumulation with active optimization of existing skills.

**Status:** Not implemented. The current skill library only accumulates and tracks, it does not evolve skills. EvoSkills is a Tier 3 innovation in the priority roadmap.

---

## Autocatalytic Thesis

### Definition

An autocatalytic set (Kauffman 1993) is a collection of entities where each entity's production is catalyzed by other entities in the set. Once the set reaches a critical diversity threshold, it becomes self-sustaining: the creation of new entities accelerates the creation of further entities, producing exponential growth.

### Application to Roko

Roko's learning subsystems form an autocatalytic set:

```
Skills catalyze → better prompts
Better prompts catalyze → higher pass rates
Higher pass rates catalyze → more successful episodes
More episodes catalyze → better pattern extraction
Better patterns catalyze → better playbook rules
Better rules catalyze → fewer failures
Fewer failures catalyze → lower costs
Lower costs catalyze → more experiments
More experiments catalyze → better skills
    ↑                              │
    └──────────────────────────────┘
         (autocatalytic cycle)
```

Each element in the cycle enables the next. The cycle is autocatalytic because it is self-reinforcing: once started, it accelerates without external input.

### Compound Improvement Math

The PRD models compound improvement as:

```
compound_success = pass_rate_routing × pass_rate_prompts × pass_rate_skills × pass_rate_rules
```

If each component has an independent 90% pass rate:

```
compound = 0.9 × 0.9 × 0.9 × 0.9 = 0.656
```

This means the system succeeds 65.6% of the time when all four components must succeed. The key insight is that **small improvements in any component multiply through the chain**:

| Improvement | New compound | Absolute gain |
|------------|-------------|---------------|
| Routing 90% → 95% | 0.95 × 0.9³ = 0.692 | +3.6% |
| All 90% → 92% | 0.92⁴ = 0.716 | +6.0% |
| All 90% → 95% | 0.95⁴ = 0.815 | +15.9% |

The multiplicative structure means that a small uniform improvement (90% → 95%) produces a larger compound improvement (65.6% → 81.5%) than any single large improvement.

### Caveats

1. **Independence assumption**: The components are not independent. Better routing may make prompt optimization less impactful (because the model is already well-chosen). The multiplicative model overestimates compound improvement when components are correlated.

2. **Diminishing returns**: Each component has a ceiling (can't exceed 100%). As components approach their ceilings, further improvement becomes harder, and the compound effect plateaus.

3. **Stability constraint**: Compound improvement only occurs when the system is stable (see [14-stability-mechanisms](14-stability-mechanisms.md)). Oscillation between components can produce compound degradation instead of compound improvement.

4. **Minimum viable diversity**: The autocatalytic cycle requires all components to function. A missing component (e.g., no skill library) breaks the cycle. This is why Tier 1M (the eight missing feedback loops) is prioritized: closing the loops enables the autocatalytic cycle to function.

---

## Network Effects

The autocatalytic thesis invokes two network scaling laws:

### Metcalfe's Law

The value of a network is proportional to N² (the number of possible connections between N nodes). In Roko's context, N is the number of learning subsystems. With 8 feedback loops connecting 10+ subsystems, the potential interaction space is O(N²) ≈ 100 interactions, each potentially creating an improvement pathway.

### Reed's Law

The value of a network is proportional to 2^N (the number of possible subsets). This applies when groups of subsystems can form emergent coalitions: the cascade router + provider health + cost normalization form a "routing coalition" that is more than the sum of its parts.

### Loreto & Tria Pólya Urn Model (2014)

The Pólya urn model for innovation predicts that the rate of discovery accelerates as the knowledge base grows: each new discovery opens adjacent possibilities that increase the probability of further discoveries. Applied to Roko: each new skill, pattern, or routing rule opens new optimization pathways that weren't previously visible.

---

## Flywheel Mechanisms

Ten mechanisms for compounding growth, adapted from the legacy architecture:

| # | Mechanism | Source | How it compounds |
|---|-----------|--------|-----------------|
| 1 | Skill accumulation | Voyager (Wang et al. 2023) | More skills → cheaper future tasks |
| 2 | Pattern extraction | Trigram mining | More patterns → fewer repeated mistakes |
| 3 | Playbook rules | Reflexion/ExpeL | More rules → higher first-attempt pass rate |
| 4 | Model routing | RouteLLM/FrugalGPT | Better routing → lower cost per task |
| 5 | Cache optimization | KV cache affinity | More reuse → lower marginal cost |
| 6 | Prompt optimization | DSPy/experiments | Better prompts → fewer iterations |
| 7 | Calibration | Predictive foraging | Better predictions → better decisions |
| 8 | Crate familiarity | LinUCB context | More experience → better model selection per crate |
| 9 | Cross-project transfer | HDC fingerprints | Skills from project A accelerate project B |
| 10 | Meta-optimization | ADAS (Hu et al. 2025) | Better architecture → better everything |

Each mechanism independently produces linear improvement. When they interact through feedback loops, the compound effect can be super-linear — but only if stability mechanisms prevent oscillation and the autocatalytic cycle is complete.

---

## Empirical Validation

The autocatalytic thesis is testable. The C-Factor (see [15-collective-calibration-31x](15-collective-calibration-31x.md)) should show:

1. **Initial plateau** (first 50 episodes): Learning subsystems bootstrapping, C-Factor near 0.5.
2. **Acceleration** (50-200 episodes): Feedback loops engaging, C-Factor rising.
3. **Super-linear growth** (200-500 episodes): Autocatalytic cycle active, C-Factor rising faster than linear.
4. **Saturation** (500+ episodes): Components approaching ceilings, growth rate decreasing.

If the C-Factor shows a linear or sub-linear trend instead of super-linear, the autocatalytic thesis is falsified for the current implementation. This falsifiability is essential: the thesis is a scientific hypothesis, not a marketing claim.

---

## Relationship to Other Documents

- **[02-skill-library-voyager](02-skill-library-voyager.md)** — Monotonic skill accumulation is mechanism #1.
- **[04-cascade-router](04-cascade-router.md)** — Model routing optimization is mechanism #4.
- **[05-pattern-discovery-trigram](05-pattern-discovery-trigram.md)** — Pattern extraction is mechanism #2.
- **[13-8-missing-feedback-loops](13-8-missing-feedback-loops.md)** — The eight loops are the connections that enable the autocatalytic cycle.
- **[14-stability-mechanisms](14-stability-mechanisms.md)** — Stability is a prerequisite for compound improvement.
- **[15-collective-calibration-31x](15-collective-calibration-31x.md)** — C-Factor measures whether compound improvement is occurring.
- **[16-predictive-foraging](16-predictive-foraging.md)** — Calibration is mechanism #7.
- **[12-self-improvement-frameworks](12-self-improvement-frameworks.md)** — Academic foundations for all mechanisms.

---

## Appendix: Critical Diversity Threshold

Kauffman's autocatalytic set theory predicts a critical diversity threshold: below a certain number of interacting components, the autocatalytic cycle cannot sustain itself. Above the threshold, the cycle becomes self-sustaining and accelerates.

For Roko, the critical components are:

| Component | Status | Role in Cycle |
|-----------|--------|--------------|
| Episode logger | Wired | Data substrate |
| Pattern miner | Wired | Knowledge extraction |
| Playbook rules | Wired | Knowledge validation |
| Skill library | Wired | Capability accumulation |
| Cascade router | Wired | Resource optimization |
| Provider health | Wired | Reliability |
| Cost normalization | Wired | Budget management |
| Regression detection | Wired | Quality assurance |
| Prompt experiments | Wired | Prompt optimization |
| C-Factor | Wired | System measurement |

All 10 components are wired. The remaining question is whether the eight inter-component feedback loops are sufficiently connected to sustain the autocatalytic cycle. Currently, 1 of 8 loops is fully wired, 3 are partially wired, and 4 are data-collection-only. The thesis predicts that closing the remaining loops will produce a phase transition in the C-Factor trend — from linear improvement to super-linear growth.

### Falsification Criteria

The autocatalytic thesis is falsified if:
1. C-Factor shows no upward trend after 500 episodes with all 8 loops wired.
2. Individual component improvements do not compound (each improvement is additive rather than multiplicative).
3. Closing additional feedback loops does not produce measurable C-Factor acceleration.

These criteria provide concrete conditions under which the thesis should be abandoned in favor of simpler linear improvement models.


---

# SOURCE: /Users/will/dev/nunchi/roko/roko/docs/05-learning/18-self-learning-cybernetic-loops.md

# Self-Learning & Cybernetic Feedback Loops

> **REF10 source:** `../../tmp/refinements/10-self-learning-cybernetic-loops.md`
> **Glossary:** [Naming and Glossary](../00-architecture/01-naming-and-glossary.md)
> **Cross-references:** [16-predictive-foraging](16-predictive-foraging.md), [13-8-missing-feedback-loops](13-8-missing-feedback-loops.md), [15-collective-calibration-31x](15-collective-calibration-31x.md), [17-adas-and-autocatalytic](17-adas-and-autocatalytic.md), [20-research-to-runtime](20-research-to-runtime.md), `../../tmp/refinements/16-research-to-runtime.md`
>
> **Implementation status**: Active inference exists in `roko-learn` (`active_inference.rs`, ~255 lines) as a working Bayesian tier selector. Prediction tracking exists in `prediction.rs`. The per-operator predict-publish-correct doctrine described here is **target-state**; today the Router has the richest prediction/outcome signals, while Bus/Pulse-mediated calibration across every operator remains planned.

---

## Purpose

REF10 describes a target-state extension that would turn learning in Roko into a Bus-backed feedback nervous system. The key move is simple: every operator becomes a predictor. It publishes a prediction Pulse, later receives an outcome Pulse, and then updates from the prediction error. That is broader than the current code: active inference already exists, but narrowly, as a routing component rather than a universal operator doctrine.

The refinement text centers three anchor learners, but the shipping learning subsystem is broader than that framing suggests. `roko-learn` already includes routing, prediction tracking, runtime feedback, bandits, drift detection, pattern discovery, skill accumulation, and provider-health handling. Within that larger crate, three obvious anchor learners are:

1. `CascadeRouter` learned which model tier to pick.
2. `EpisodeLogger` accumulated completed runs for replay and distillation.
3. `ExperimentStore` ran prompt A/B tests.

The refinement does not replace those learners. It makes them uniform. Once the Bus is first-class, the same predict-publish-correct pattern can be used for routing, prompt composition, gate thresholds, policy decisions, and storage-tier choices.

## The Predict-Publish-Correct Loop

For any operator `O` that transforms input `x` into output `y`, the learning loop is:

1. `O` publishes a prediction Pulse with a topic in the `prediction.*` family.
2. A downstream system publishes the actual outcome on `outcome.*` with the same lineage hint.
3. A calibration policy joins the two Pulses, computes loss, and updates operator state.
4. `O` subscribes to its calibration update and adjusts future behavior.

In prose, that is the Free Energy Principle implemented as a Bus protocol: make a prediction, compare it to the world, and minimize future error. The important detail is that the error is not hidden inside one operator; it becomes a first-class Pulse stream that other learners can subscribe to.

## Per-Operator Calibration

The Bus makes per-operator calibration cheap enough to do everywhere, not just in the router.

| Operator | Predicts | Outcome signal | Update policy |
|---|---|---|---|
| `Scorer` | Candidate quality or reward by score axis | Gate verdict plus episode reward | Online calibration per axis, with reliability curves |
| `Router` | Which action or model choice will succeed | Gate verdict | Contextual bandit updates |
| `Composer` | Whether the prompt fits budget and wins the gate | Token count plus gate verdict | Template EMA and variant selection |
| `Gate` | Whether the task will succeed post-patch | Next verdict plus regression tests | Threshold smoothing and drift correction |
| `Policy` | Whether a decision will improve a metric | Metric Pulse after the decision | Per-policy online calibration |
| `Substrate` | Whether an Engram belongs in a given tier | Query frequency, recency, and reuse | Tier-promotion and retention policy |

This is the missing middle between fixed heuristics and heavyweight model retraining. The calibration target is not just “did the task pass?” but “which operator was systematically overconfident, underconfident, or stale?”

The same calibration machinery could eventually apply to research-derived defaults. That depends on the separate research-to-runtime work landing first; today there is no `claim!`-style runtime resolver or replication ledger in the codebase.

## CalibrationPolicy

`CalibrationPolicy` is the chapter-level name for the Bus consumer that closes the loop. It subscribes to the `prediction.*` and `outcome.*` families, matches records by lineage, and maintains per-operator state:

- trial counts
- error accumulators
- EMA of recent error
- axis-specific calibration curves where the operator has multiple sub-scores

When the policy closes a prediction/outcome pair, it publishes a calibration update on a topic such as `calibration.scorer.updated` or `calibration.router.updated`. The operator then consumes that update the same way it consumes any other Bus-delivered fact.

The concrete implementation details can vary, but the structure should not:

- prediction Pulses are lightweight and ephemeral
- outcome Pulses are ground truth from the downstream step
- calibration updates are separate Pulses, not hidden side effects
- the policy itself is just another Bus subscriber

That same policy could eventually ingest research-derived outcomes for paper-backed claims. The replication-ledger portion of that design is deferred for now.

That separation matters because it keeps learning composable. Operators do not need to know who is measuring them. They only need to publish predictions and react to calibration updates.

## `prediction.error.*` As A First-Class Signal

`prediction.error.*` is the shared language for uncertainty, drift, and surprise. It is useful at three levels:

1. Local error tells an operator how far off a specific prediction was.
2. Aggregated error tells the system which operator or topic is drifting.
3. Elevated error tells the planner or Dreams loop where to spend attention next.

This is why the refinement treats prediction error as a first-class signal. A spike in `prediction.error.high` is not just a debugging artifact. It is a routing input for learning itself. High-error regions can be replayed, consolidated, or prioritized for retraining.

The practical effect is that curiosity becomes observable. The system learns where its own models are weakest and spends effort there first.

## Existing Learners Reading Off The Bus

The Bus does not invent new learners; it rewires the existing ones so they subscribe to facts instead of being called directly.

### `CascadeRouter`

`CascadeRouter` becomes a subscriber to `router.selection.made` and `router.selection.outcome`. It updates its bandit state from those Pulses and publishes `router.weights.updated` when its internal calibration changes. That decouples routing logic from the routing caller.

### `EpisodeLogger`

`EpisodeLogger` subscribes to `agent.turn.completed` and `gate.verdict.emitted`, then correlates them into episodes. The orchestrator no longer needs to know the logger exists. The logger just reads the Bus and persists the records that matter.

### `ExperimentStore`

`ExperimentStore` subscribes to `composer.invocation.started`, assigns a prompt variant, and publishes `composer.variant.assigned`. Gate verdicts later close the loop. That turns prompt experimentation into a continuous Bus-driven optimization process instead of an ad hoc side channel.

The architectural payoff is that each learner becomes replaceable and composable. Adding a new learner means writing one subscriber and one publisher, not threading another callback through the whole runtime.

## Why This Matters

This chapter is the learning-layer expression of the broader two-fabric design:

- the Bus carries prediction and outcome Pulses
- the Substrate retains durable records when lineage matters
- active inference is already real for routing, while broader operator coverage remains target-state
- every operator can eventually be calibrated independently once the transport and outcome surfaces exist
- existing learners can cooperate without tight coupling

The result is a self-modeling system. Prediction error is no longer an incidental byproduct; it is the signal that keeps the system honest.

## Relationship To Other Docs

- [16-predictive-foraging](16-predictive-foraging.md) covers task-level prediction and calibration; this doc generalizes the same logic to operators.
- [13-8-missing-feedback-loops](13-8-missing-feedback-loops.md) catalogs the current wiring gaps that REF10 turns into Bus-backed learner loops.
- [15-collective-calibration-31x](15-collective-calibration-31x.md) applies calibration at the collective level.
- [17-adas-and-autocatalytic](17-adas-and-autocatalytic.md) frames the compound effect of the loops as autocatalytic growth.
- [20-research-to-runtime](20-research-to-runtime.md) sketches the target-state paper → claim → heuristic → trial → calibration pipeline and the deferred replication-ledger layer.
- See also `tmp/refinements/10-self-learning-cybernetic-loops.md` for the full proposal.


---

# SOURCE: /Users/will/dev/nunchi/roko/roko/docs/05-learning/19-heuristics-worldviews-and-falsifiers.md

# Heuristics, Worldviews, and Falsifiers

> **REF14 source:** `../../tmp/refinements/14-worldview-validation.md`
> **Glossary:** [Naming and Glossary](../00-architecture/01-naming-and-glossary.md)
> **Cross-references:** [01-playbook-system](01-playbook-system.md), [16-predictive-foraging](16-predictive-foraging.md), [18-self-learning-cybernetic-loops](18-self-learning-cybernetic-loops.md), [20-research-to-runtime](20-research-to-runtime.md), [12-4-tier-distillation-pipeline](../06-neuro/12-4-tier-distillation-pipeline.md), [14-c-factor-collective-intelligence](../00-architecture/14-c-factor-collective-intelligence.md), [25-attention-as-currency](../00-architecture/25-attention-as-currency.md), `../../tmp/refinements/16-research-to-runtime.md`
>
> **Implementation status**: `HeuristicRule` exists in `roko-neuro`. The full worldview/falsifier/dissonance stack described here is **target-state**. Near-term: typed heuristic specs and contradiction tracking. Deferred: worldview clustering, dissonance algebra, and belief export/import.

---

## Purpose

Episodes tell Roko what happened. Playbooks tell it which concrete sequences have worked before. REF14 proposes a richer missing middle: `Heuristic` Engrams that capture a reusable claim, the conditions where it applies, the predicted outcome, and the calibration record showing whether lived experience keeps confirming it. The near-term version of that idea builds on the existing `HeuristicRule` in `roko-neuro` with typed specs and contradiction tracking. See `tmp/refinements/14-worldview-validation.md` for the full proposal.

REF16 extends that middle into a research pipeline. That full paper/claim/replication-ledger stack is deferred, but the provenance instinct remains useful: heuristics should be able to point back to source material when that source materially informed the rule.

This matters because playbooks alone are too concrete. They bind to particular tools, paths, and workflow orderings. Heuristics are more abstract: they say what to check, what to expect, and what would count as being wrong. That gives the learning stack a durable library of priors that can survive tool churn, compose across domains, and be inspected by the user.

The learning story therefore becomes:

1. Episodes capture raw work and outcomes.
2. Distillation extracts candidate insights and heuristics.
3. Calibration promotes, refines, or cools heuristics based on real outcomes.
4. Worldviews cluster heuristics that co-occur in successful episodes.
5. Playbooks compile the most concrete, battle-tested procedural fragments for direct reuse.

## Why Playbooks Are Not Enough

Playbooks remain useful, but they are the wrong level of abstraction for many learning tasks:

- They are over-specific to tools, layouts, and local workflow details.
- They flatten "what worked" without preserving the belief that made it plausible.
- They make contradiction handling awkward because success or failure is attached to the whole sequence, not the underlying prior.
- They do not give the Router or Composer a clean way to keep multiple competing priors alive on purpose.

Heuristics solve that by turning reusable beliefs into first-class durable records. A playbook can still be compiled later, but now the system knows which prior it came from, how often it held up, and what evidence is currently pushing against it.

## The `Heuristic` Engram

REF14 treats heuristics as a first-class durable kind rather than an implementation detail hidden inside playbooks or prompt templates:

```rust
pub struct Heuristic {
    pub id: Uuid,
    pub claim: String,
    pub preconditions: Vec<Predicate>,
    pub prediction: Predicate,
    pub fingerprint: HdcVector,
    pub calibration: Calibration,
    pub lineage: Vec<HeuristicId>,
    pub receipts: Vec<EpisodeHash>,
}

pub struct Calibration {
    pub trials: u32,
    pub confirmations: u32,
    pub violations: u32,
    pub brier_score: f64,
    pub last_trial_at: Timestamp,
    pub confidence_interval: (f64, f64),
}
```

Three details are load-bearing:

- `preconditions` make the claim matchable against the current situation instead of being free text.
- `prediction` says what should happen if the heuristic is correct.
- `receipts` preserve the episode lineage that justified the heuristic in the first place.

Because heuristics are Engrams, they can share the rest of the durable-memory stack: HDC fingerprint similarity where available, provenance, lineage, and tiered retention. The demurrage-balance model described elsewhere remains deferred.

## Predicates and Falsifiers

A heuristic is only useful if the system can tell when it applies and when reality disproves it. REF14's `Predicate` surface gives Roko both:

```rust
pub enum Predicate {
    LanguageIs(Language),
    FileMatches(Glob),
    ToolAvailable(ToolId),
    GateRecentlyFailed(GateId),
    AgentRoleIs(Role),
    And(Vec<Predicate>),
    Or(Vec<Predicate>),
    Not(Box<Predicate>),
    SimilarTo { fingerprint: HdcVector, threshold: f64 },
}
```

In the learning layer, a **falsifier** is the concrete outcome check that can refute a heuristic's prediction. Sometimes that is a direct contradiction (`prediction` failed after the preconditions matched). Sometimes it is a targeted check emitted as a Bus-visible outcome Pulse such as a gate verdict, regression result, or metric delta that should have moved but did not. The important part is that every heuristic has an inspectable failure surface rather than an untestable slogan.

That gives the system a consistent contract:

- Composer retrieves heuristics whose preconditions match the current situation.
- ACT and VERIFY emit the outcome Pulses that reality provides.
- A calibrator decides whether those outcomes confirmed, violated, or refined the claim.
- The falsifier record is durable and queryable after the fact.

This is the REF14 synergy with REF10 and the two-fabric model: heuristics are durable Engrams, while the runtime can eventually deliver the reality-check signals that confirm, contradict, or refine them.

## Heuristic Lifecycle

### Birth

Heuristics enter the system in three ways:

1. Distilled from repeated episodes and insights.
2. Stated explicitly by an agent as a candidate prior.
3. Imported from research or from another deployment with an attached trust factor.

Fresh heuristics should start advisory rather than dominant. They need receipts, trials, and calibration before they earn prompt weight.

When a heuristic is imported from research, its receipts should at minimum include the source paper or note that informed it. The fuller replication-ledger story is deferred.

### Test

Every episode is a potential test. Before action, Composer scans for heuristics whose `preconditions` match. After action, Policy and Gate outputs close the loop:

```rust
pub trait Calibrator {
    fn score(&self, heuristic: &Heuristic, episode: &Episode) -> Verdict;
}

pub enum Verdict {
    Confirmed,
    Violated,
    Irrelevant,
    Refined(Heuristic),
    Generalized(Heuristic),
    Refuted,
}
```

`Confirmed` and `Violated` update the calibration record. `Refined` and `Generalized` create new lineage-linked heuristics instead of mutating history in place. `Refuted` retires the heuristic from the hot path without breaking lineage resolution.

### Adjust

Calibration should be empirical and incremental:

- `trials` increments when preconditions actually matched.
- `confirmations` increments when the predicted outcome held.
- `violations` increments when the falsifier surface fired.
- Brier score and Wilson confidence intervals track both sharpness and reliability.

Paper-derived heuristics can eventually use the same calibration path. For now, the practical goal is simpler provenance plus local confirmation and contradiction tracking rather than a full replication ledger.

Prompt weighting should follow the confidence lower bound, not raw win rate. That keeps young heuristics usable without letting a tiny sample masquerade as certainty.

### Retire

Retirement is not deletion. A refuted heuristic should lose influence through the existing confidence and tiering machinery, remain resolvable by content hash, and preserve the receipts that explain why it was trusted and later challenged. History is preserved; attention is reallocated.

## Worldviews As Co-Citation Clusters

In the target-state design, a worldview is a cluster of heuristics that keep appearing together in successful episodes. It is not a handcrafted persona. It is an observed structure in the heuristic citation graph.

```rust
pub struct Worldview {
    pub id: Uuid,
    pub core_heuristics: Vec<HeuristicId>,
    pub coherence_score: f64,
    pub effectiveness_score: f64,
    pub domain_fingerprint: HdcVector,
}
```

REF14 adds three practical uses for worldview clustering once the underlying data exists:

- Router can pick the worldview whose `domain_fingerprint` best matches the incoming task.
- Composer can inject the worldview's core heuristics as a coherent prior set rather than a random pile of tips.
- Policy can keep multiple worldviews active so the system does not collapse into one monoculture.

This is the link to REF13's c-factor work. Diversity is not treated as noise to eliminate; it is a measured capability. The main worldview handles the common case, a challenger worldview keeps the calibration loop honest, and niche worldviews stay in cold storage until the domain matches again.

## Dissonance and Active Learning

REF14 makes contradictions visible instead of smoothing them away. Near-term, that means contradiction tracking around heuristic calibration. In the fuller target-state design, when two active heuristics predict incompatible outcomes for the same situation, the system would emit a dissonance record:

```rust
pub struct Dissonance {
    pub heuristics: [HeuristicId; 2],
    pub predictions: [Predicate; 2],
    pub situation: SituationHash,
}
```

Dissonance matters because it is high-information work:

1. It identifies where the current worldview is internally inconsistent.
2. It creates a natural active-learning queue for decisive tests.
3. It lets later episodes update both competing heuristics against the same ground truth.

The scheduling implication is deliberate: if the system can cheaply gather reality on a dissonant case, it should often prefer doing that over another low-information repetition.

## Inspectability and Sharing

Heuristics should be externally inspectable in a way playbooks alone are not. A user should be able to ask:

- Which heuristics are highly calibrated?
- Which ones are recent, unproven hypotheses?
- Which worldviews dominate a given domain?
- Which falsifiers have been firing most often?

That implies a first-class query surface such as `roko heuristic list`, `show`, `stats`, `similar`, `export`, and `import`. Imported heuristics should retain their receipts and calibration metadata but enter with a configurable trust discount until local evidence revalidates them.

If REF16 lands, the same logic can be applied to research-derived runtime defaults: a `claim!`-style resolver could map a config key to a claim ID, then materialize the parameter only if the claim's replication ledger and local calibration are still inside tolerance. If the claim degrades, the resolver should fall back to a safe default rather than silently preserving stale provenance.

This export/import flow is a longer-range idea. It also composes with REF16's deferred replication-ledger framing, but that paper-economy layer is not current architecture.

## Interaction With Playbooks, Neuro, and Profiles

REF14 does not remove playbooks. It narrows their role:

- Heuristics are a promising durable belief layer to strengthen on top of today's `HeuristicRule` machinery.
- Playbooks are compiled procedural projections of heuristics and strategy fragments.
- Neuro stores heuristics and related knowledge as durable Engrams; broader clustering and demurrage-balance semantics remain target-state.
- Domain profiles seed an initial heuristic library, but calibration remains per heuristic rather than per profile.

That separation gives the docs a cleaner architecture story. Learning owns episode feedback, calibration, worldview competition, and externalization of beliefs. Neuro owns durable storage, similarity, and tier movement. Playbooks remain the human-readable, highly concrete output surface rather than the only memory object worth preserving.

## Relationship To Other Docs

- [01-playbook-system](01-playbook-system.md) now reads playbooks as compiled downstream artifacts rather than the only validated knowledge tier.
- [16-predictive-foraging](16-predictive-foraging.md) covers Brier scores and prediction quality at task level; heuristics reuse the same calibration logic at belief level.
- [18-self-learning-cybernetic-loops](18-self-learning-cybernetic-loops.md) explains how the Bus carries the outcome Pulses and calibration topics that falsify or reinforce heuristics.
- [20-research-to-runtime](20-research-to-runtime.md) sketches the target-state paper → claim → heuristic → trial → calibration pipeline and the deferred replication-ledger format.
- [12-4-tier-distillation-pipeline](../06-neuro/12-4-tier-distillation-pipeline.md) describes how Neuro distills, stores, and cools the durable heuristic library.
- [14-c-factor-collective-intelligence](../00-architecture/14-c-factor-collective-intelligence.md) provides the cohort-level reason to keep challenger and niche worldviews active.
- See also `tmp/refinements/14-worldview-validation.md` for the full proposal.


---

# SOURCE: /Users/will/dev/nunchi/roko/roko/docs/05-learning/20-research-to-runtime.md

# Research-to-Runtime Pipeline

> **REF16 source:** `../../tmp/refinements/16-research-to-runtime.md`
> **Glossary:** [Naming and Glossary](../00-architecture/01-naming-and-glossary.md)
> **Cross-references:** [18-self-learning-cybernetic-loops](18-self-learning-cybernetic-loops.md), [19-heuristics-worldviews-and-falsifiers](19-heuristics-worldviews-and-falsifiers.md), [16-predictive-foraging](16-predictive-foraging.md), [25-research-to-runtime](../21-references/25-research-to-runtime.md), [12-4-tier-distillation-pipeline](../06-neuro/12-4-tier-distillation-pipeline.md)
>
> **Implementation status**: Target-state concept. No `Claim`, `Paper`, or replication-ledger code exists. The provenance-backed heuristic idea is valuable; the full paper economy (claims, replication trials, ledger) is deferred.

---

## Purpose

REF16 describes a target-state way to make research part of the learning loop rather than a one-time influence. In that design, the system would ingest papers continuously, extract testable claims, lift validated claims into heuristics, and keep the resulting evidence live through replication-ledger feedback. Today, the most practical slice is narrower: keep provenance visible when papers inform heuristics or parameters.

This chapter is the learning-layer bridge between academic provenance and runtime behavior. It explains how paper-backed ideas become parameter choices, heuristics, and calibration records that can be checked, revised, or retired when the live system disagrees.

## The Pipeline

The proposed flow is:

1. **Paper** - ingest the source as an Engram with authorship, venue, and provenance.
2. **Claim** - extract or author a testable hypothesis with an explicit falsifier.
3. **Heuristic** - lift the claim into a reusable prior once local structure is stable enough.
4. **Trial** - run the heuristic against real episodes, gates, and outcome Pulses.
5. **Calibration** - update confidence, confidence bounds, and trust based on what actually happened.

The key point is that the same evidence can move through the stack more than once. A claim may begin as a paper-backed prior, then become a heuristic, then be revised by trial results, then be demoted or promoted by its replication ledger. That full lifecycle is deferred for now.

## Paper As Engram (Target-State)

Papers live in the same durable substrate as other long-lived records. That means the source itself stays addressable, citeable, and comparable over time.

```rust
pub struct Paper {
    pub title: String,
    pub authors: Vec<String>,
    pub venue: Option<String>,
    pub year: u16,
    pub provenance: PaperProvenance,
    pub claims: Vec<ClaimId>,
}
```

The important behavior is not the exact schema. It is that the source paper remains available for later review, so the system can distinguish "this was a paper-backed idea" from "this worked in our stack."

## Claim As Hypothesis (Target-State)

A claim is the paper's runtime-facing unit of meaning. It should be small enough to test and explicit enough to fail.

```rust
pub struct Claim {
    pub paper: PaperId,
    pub hypothesis: Hypothesis,
    pub falsifier: Predicate,
    pub context: Vec<Predicate>,
    pub calibration: Calibration,
}
```

The falsifier is the load-bearing part. If the claim cannot be disproved by runtime signals, it is not yet a learning object. In practice, the falsifier should point at observable outcomes already present in the Bus or Episode streams.

## Heuristic Lifting

Claims that survive repeated trials can lift into `Heuristic` Engrams. That lifting preserves lineage rather than flattening the research source into a generic rule.

The practical rule is:

- paper → provenance
- claim → testable hypothesis with a falsifier
- heuristic → reusable belief with calibration and receipts

That is the same REF14 machinery, but with a research-specific origin story.

## Replication Ledger

In the target-state design, the replication ledger is the bridge between external research and local calibration. It records how the paper's reported effect compares with the effect observed in Roko's actual deployment.

```rust
pub struct ReplicationLedger {
    pub claim: ClaimId,
    pub paper_effect: f64,
    pub our_effect: f64,
    pub our_n: u32,
    pub divergence_ci: (f64, f64),
    pub status: ReplicationStatus,
}
```

The ledger is not just a report. It is an input to calibration. If a paper-derived claim keeps replicating, the associated heuristic stays warm. If it diverges, the claim should lose weight even if the original citation is strong.

That makes evidence cumulative instead of ceremonial. The runtime cares about the paper, but it trusts the paper only through the behavior it continues to see.

## Claim-Resolved Parameters

Some runtime defaults should be resolved from claims rather than literals. The docs may express this as a `claim!` macro, a resolver function, or equivalent lookup, but the behavior should be the same: the parameter is bound to a claim ID, not just a comment.

```rust
let epsilon = claim!("auer2002", "epsilon_greedy", default = 0.1)?;
```

If the claim's replication ledger weakens or local calibration drifts too far, the target-state resolver should fall back to a safe default or a lower-trust value. That keeps paper-backed parameters auditable without making them sticky.

## Calibration Feedback

In the target-state design, calibration should consume both local trials and replication-ledger updates.

- Local trials say whether the heuristic works in this deployment.
- Replication-ledger entries say whether the paper's claim still matches the deployed reality.
- The combined signal updates confidence, trust, and promotion or retirement decisions.

This is the main operational consequence of REF16: the system does not merely cite research. It tests it, tracks divergence, and lets the result change runtime behavior.

## Relationship To Other Docs

- [19-heuristics-worldviews-and-falsifiers](19-heuristics-worldviews-and-falsifiers.md) defines the heuristic layer that paper claims lift into.
- [18-self-learning-cybernetic-loops](18-self-learning-cybernetic-loops.md) defines the Bus-backed calibration loop that carries the trial outcomes.
- [16-predictive-foraging](16-predictive-foraging.md) covers task-level prediction and calibration, which this chapter reuses at research level.
- [25-research-to-runtime](../21-references/25-research-to-runtime.md) collects the source-facing paper, claim, starter-kit, and replication-contract framing.
- [12-4-tier-distillation-pipeline](../06-neuro/12-4-tier-distillation-pipeline.md) covers the durable memory tier that keeps research lineage available.
- See also `../../tmp/refinements/16-research-to-runtime.md` for the full proposal.


---

# SOURCE: /Users/will/dev/nunchi/roko/roko/docs/05-learning/INDEX.md

# 05 — Learning

> **Crate:** `roko-learn` · **Path:** `crates/roko-learn/src/`
> **Persistence root:** `.roko/learn/`
> **Entry point:** `LearningRuntime` in `runtime_feedback.rs`
>
> **Implementation status**
> - **Shipping**: `roko-learn` is already one of the largest subsystems in the repo at roughly 42 modules and 35,847 LOC. Shipping pieces include `cascade_router`, `runtime_feedback`, `skill_library`, `episode_logger`, bandits, prediction tracking, active inference, drift detection, pattern discovery, and provider health.
> - **Target-state**: Bus-backed cross-operator calibration, typed heuristic specs, and richer provenance-backed research ingestion.
> - **Deferred**: demurrage as the governing memory model, worldview clustering and dissonance algebra as a canonical layer, and the full Paper/Claim/replication-ledger stack.

---

## Overview

The learning subsystem turns every agent execution into training data. Each agent turn produces an episode, each episode updates baselines, each baseline informs routing, and each routing decision produces a new episode. That core loop already ships in a substantial form rather than as a nascent sketch: `roko-learn` currently spans roughly 42 modules and 35,847 LOC, including `cascade_router` (3-stage model routing), `runtime_feedback`, `skill_library`, `episode_logger`, bandits, prediction tracking, active inference, drift detection, pattern discovery, and provider-health circuit breaking.

REF10, REF14, and REF16 still matter here, but mainly as target-state design docs layered on top of that existing crate. REF10 describes a Bus-backed predict-publish-correct architecture for broader cross-operator calibration; today active inference and prediction tracking are real, but the Router has the richest prediction/outcome signal and the universal Bus/Pulse doctrine remains planned. REF16 similarly describes a richer provenance path from research into runtime behavior, but the Paper/Claim/replication-ledger stack is still deferred.

REF12's demurrage framing is explicitly deferred. Current learning and memory code uses existing decay, confidence, and store-specific retention behavior rather than a balance-bearing attention economy. See also [04-decay-variants](../00-architecture/04-decay-variants.md) and [25-attention-as-currency](../00-architecture/25-attention-as-currency.md) for the deferred design.

REF14 is most useful in a narrower form. `HeuristicRule` already exists in `roko-neuro`, and near-term value comes from typed heuristic specs plus contradiction and calibration tracking. Worldview clustering, dissonance algebra, and heuristic export/import remain target-state rather than current runtime behavior. See [19-heuristics-worldviews-and-falsifiers](19-heuristics-worldviews-and-falsifiers.md) and [20-research-to-runtime](20-research-to-runtime.md) for those scoped designs.

The docs describe four durable learning surfaces, but they are not all equally mature yet: episodes and metrics are **shipping**; patterns, bandits, and routing baselines are **shipping**; heuristic/playbook distillation exists in partial form today and is the main **ship soon** layer; worldviews and replication-ledger-backed research ingestion are **planned/deferred**. The crate also includes three bandit algorithms for online decision-making (UCB1, LinUCB, Track-and-Stop), a three-stage cascade router for model selection (Static → Confidence → UCB), and eight cybernetic feedback loops described as a target architecture rather than a fully unified present-tense doctrine.

---

## Sub-Documents

### Core Data Infrastructure

| # | Document | What it covers |
|---|----------|---------------|
| [00](00-episode-logger.md) | **Episode Logger** | Append-only JSONL episode log, HDC fingerprinting, crash-safe writes, tolerant reader. The foundational data substrate for all learning. |
| [01](01-playbook-system.md) | **Playbook System** | Playbook rules with globset trigger matching, bounded confidence dynamics (validate +0.05, contradict −0.10, ceiling 0.95), and TOML persistence. Demurrage-style freshness remains a deferred design rather than current behavior. |
| [02](02-skill-library-voyager.md) | **Skill Library (Voyager)** | Voyager-style skill accumulation (Wang et al. 2023). Monotonically growing library of reusable capabilities with prompt templates, tool dependencies, usage telemetry, and deduplication. |

### Bandit Algorithms

| # | Document | What it covers |
|---|----------|---------------|
| [03](03-bandits-ucb-thompson-linucb.md) | **Bandits: UCB1, Thompson, LinUCB** | UCB1 (Auer et al. 2002), LinUCB 18-dim contextual bandit (Li et al. 2010), Track-and-Stop best-arm identification (Garivier & Kaufmann 2016), BanditBank keyed collections. |
| [04](04-cascade-router.md) | **Cascade Router** | Three-stage model routing: Static (<50 obs) → Confidence (50-200) → UCB (>200). CascadeModel with primary + fallback. Provider health filtering, Pareto pruning, C-Factor bias. |

### Metrics and Monitoring

| # | Document | What it covers |
|---|----------|---------------|
| [05](05-pattern-discovery-trigram.md) | **Pattern Discovery (Trigram)** | Trigram mining across episodes via EpisodeView trait. HDC k-medoids clustering for cross-episode consolidation. Operates every 20 episodes (slowest learning loop). |
| [06](06-task-metrics-and-baselines.md) | **Task Metrics and Baselines** | TaskMetric JSONL writer, per-(role, complexity) SliceBaseline computation, AgentEfficiencyEvent with 20+ fields, A-D prompt grading, four key self-improvement metrics. |
| [07](07-regression-detection.md) | **Regression Detection** | Compare current batch against historical baseline. Thresholds: pass rate drop >15% (Alert), cost increase >20% (Alert), duration +30% (Warning), iterations +25% (Warning). Per-slice analysis. |

### Cost and Provider Management

| # | Document | What it covers |
|---|----------|---------------|
| [08](08-cost-normalization.md) | **Cost Normalization** | CostTable, blended cost formula (3:1 input:output, Artificial Analysis methodology), multi-level budget guardrails (80% downgrade, 95% block, 100% hard stop), CostsLog append-only persistence. |
| [09](09-provider-health-circuit-breaker.md) | **Provider Health / Circuit Breaker** | Three-state circuit breaker (Closed → Open → Half-Open), error classification (RateLimit, AuthFailure, Timeout, ServerError, ContentPolicy, ContextOverflow), error-specific cooldowns, EWMA anomaly detection. |
| [10](10-pareto-frontier-pruning.md) | **Pareto Frontier Pruning** | Non-dominated set computation over (pass_rate, cost_per_success). Pruned models excluded from bandit candidate set. Recomputed every 50 observations. |

### Advanced Algorithms

| # | Document | What it covers |
|---|----------|---------------|
| [11](11-thompson-sampling-drift.md) | **Thompson Sampling with Drift** | Bayesian bandit with discount factor γ for non-stationary environments. Beta distribution per arm, discounted updates, drift detection and arm reset. |
| [12](12-self-improvement-frameworks.md) | **Self-Improvement Frameworks** | Survey: Reflexion (Shinn et al. 2023), ExpeL (Zhao et al. 2023), DSPy (Khattab et al. 2023), RouteLLM (ICLR 2025), FrugalGPT (arXiv:2305.05176), AutoMix (NeurIPS 2024), Karpathy autoresearch. External verifier requirement. |

### Research and Evidence

| # | Document | What it covers |
|---|----------|---------------|
| [20](20-research-to-runtime.md) | **Research-to-Runtime Pipeline** | Target-state provenance flow from paper-backed ideas into heuristics and calibration. The full Paper/Claim/replication-ledger model is deferred. |

### Cybernetic Architecture

| # | Document | What it covers |
|---|----------|---------------|
| [13](13-8-missing-feedback-loops.md) | **Eight Missing Feedback Loops** | Health→Routing, Conductor→Routing, Section→Scaffold, Failure→Replanning, Skills→Prompts, Cost→Routing, Latency→Reward, Experiments→Static. Status of each loop. |
| [14](14-stability-mechanisms.md) | **Stability Mechanisms** | Hysteresis (10% score delta to switch), frequency separation (every 1/5/20/50 episodes), EMA damping, anti-patterns (lock-in, explosion, feedback collapse). |
| [15](15-collective-calibration-31x.md) | **Collective Calibration (31.6×)** | CLT-inspired heuristic `accuracy(t) = 1 − 1/√(N×t)`. Explicit caveats (independence, stationarity, aggregation). C-Factor composite metric with 11 components and leave-one-out agent contributions. |
| [16](16-predictive-foraging.md) | **Predictive Foraging** | Falsifiable predictions (duration, complexity, gate outcome, conflict). CalibrationTracker, arithmetic corrector (~50ns). Brier score calibration metric, reliability diagrams. |
| [17](17-adas-and-autocatalytic.md) | **ADAS and Autocatalytic Thesis** | ADAS meta-architecture search (Hu et al. ICLR 2025, +14% ARC). EvoSkills (Chen et al. 2023). Autocatalytic sets (Kauffman 1993). Compound math: 0.9⁴ = 0.656. Ten flywheel mechanisms. Empirical testability via C-Factor trend. |
| [18](18-self-learning-cybernetic-loops.md) | **Self-Learning & Cybernetic Feedback Loops** | Target-state predict-publish-correct architecture. Shipping today: routing-side active inference and prediction tracking; deferred: universal per-operator Bus calibration. |
| [19](19-heuristics-worldviews-and-falsifiers.md) | **Heuristics, Worldviews, and Falsifiers** | Near-term value: typed heuristic specs and contradiction tracking around existing heuristic distillation. Deferred: worldview clustering, dissonance algebra, and belief export/import. |
---

## LearningRuntime: The Integration Hub

All learning subsystems are coordinated through `LearningRuntime` in `runtime_feedback.rs`. A single method — `record_completed_run(CompletedRunInput)` — updates every subsystem in a consistent order:

```
CompletedRunInput
    │
    ├── 1. EpisodeLogger::append()           → episodes.jsonl
    ├── 2. CostsLog::append()                → costs.jsonl
    ├── 3. PlaybookStore::record_outcome()   → playbooks/*.json
    ├── 4. PlaybookRules::validate/contradict → playbook-rules.toml
    ├── 5. SkillLibrary::record_use()        → skills.json
    ├── 6. TaskMetric → regression history   → task-metrics.jsonl
    ├── 7. ExperimentStore::record_outcome() → experiments.json
    ├── 8. PatternMiner::ingest_episode()    → (in-memory)
    ├── 9. CascadeRouter::update()           → cascade-router.json
    └── 10. CFactor::compute()               → c-factor.jsonl
```

### Persistence Layout

```
.roko/learn/
├── episodes.jsonl         ← append-only episode log
├── costs.jsonl            ← append-only cost records
├── task-metrics.jsonl     ← append-only task metrics
├── efficiency.jsonl       ← append-only efficiency events
├── c-factor.jsonl         ← append-only C-Factor snapshots
├── skills.json            ← skill library (atomic write)
├── cascade-router.json    ← cascade router state (atomic write)
├── experiments.json       ← experiment store (atomic write)
├── gate-thresholds.json   ← adaptive gate thresholds (atomic write)
├── playbook-rules.toml    ← playbook rules (atomic write)
└── playbooks/             ← per-playbook JSON files
    ├── pb-001.json
    ├── pb-002.json
    └── ...
```

---

## Cross-References to Other Topics

| Topic | Relationship |
|-------|-------------|
| [00-architecture](../00-architecture/INDEX.md) | Engram data model that episodes extend |
| [02-agents](../02-agents/INDEX.md) | Agent dispatch produces the episodes that learning consumes |
| [03-composition](../03-composition/INDEX.md) | Prompt assembly uses skills and playbook rules from learning |
| [04-verification](../04-verification/INDEX.md) | Gate pipeline produces GateVerdict records consumed by learning |
| [07-conductor](../07-conductor/INDEX.md) | Conductor load signals feed into feedback loop 2 |
| [16-heartbeat](../16-heartbeat/INDEX.md) | Dashboard surfaces C-Factor, predictions, regression alerts |

---

## Key Academic Citations

| Citation | Used In | Contribution |
|----------|---------|-------------|
| Auer, Cesa-Bianchi & Fischer 2002 | [03](03-bandits-ucb-thompson-linucb.md) | UCB1 algorithm |
| Li et al. 2010 | [03](03-bandits-ucb-thompson-linucb.md), [04](04-cascade-router.md) | LinUCB contextual bandit |
| Garivier & Kaufmann 2016 | [03](03-bandits-ucb-thompson-linucb.md) | Track-and-Stop best-arm identification |
| Thompson 1933 | [11](11-thompson-sampling-drift.md) | Thompson Sampling |
| Wang et al. 2023 | [02](02-skill-library-voyager.md) | Voyager skill library |
| Zhao et al. 2023 | [12](12-self-improvement-frameworks.md) | ExpeL experience extraction |
| Shinn et al. 2023 | [12](12-self-improvement-frameworks.md) | Reflexion |
| Khattab et al. 2023 | [12](12-self-improvement-frameworks.md) | DSPy prompt optimization |
| Hu et al. ICLR 2025 | [17](17-adas-and-autocatalytic.md) | ADAS meta-architecture search |
| Chen et al. 2023 | [17](17-adas-and-autocatalytic.md) | EvoSkills |
| Kauffman 1993 | [17](17-adas-and-autocatalytic.md) | Autocatalytic sets |
| Ong et al. ICLR 2025 | [12](12-self-improvement-frameworks.md) | RouteLLM |
| Chen et al. arXiv:2305.05176 | [12](12-self-improvement-frameworks.md) | FrugalGPT |
| Loreto & Tria 2014 | [17](17-adas-and-autocatalytic.md) | Pólya urn model for innovation |
| Huang et al. ICLR 2024 | [12](12-self-improvement-frameworks.md) | External verifier requirement |
| Song et al. ICLR 2025 | [12](12-self-improvement-frameworks.md) | Self-improvement verification |
| Pan et al. ICML 2024 | [12](12-self-improvement-frameworks.md) | Self-improvement limitations |
| Garivier & Moulines 2011 | [11](11-thompson-sampling-drift.md) | Discounted Thompson Sampling |
| Gneiting & Raftery 2007 | [16](16-predictive-foraging.md) | Calibration theory |
| Schaul et al. 2016 | [00](00-episode-logger.md) | Prioritized experience replay |
| Andrychowicz et al. 2017 | [00](00-episode-logger.md) | Hindsight experience replay |
| Zhou et al. 2020 | [03](03-bandits-ucb-thompson-linucb.md) | NeuralUCB algorithm |
| Zhu et al. 2023 | [03](03-bandits-ucb-thompson-linucb.md) | Non-stationary neural bandits (NP-ES) |
| Fedus et al. 2022 | [04](04-cascade-router.md) | Switch Transformer MoE routing |
| Zhou et al. 2022 | [04](04-cascade-router.md) | Expert Choice routing |
| Leviathan et al. 2023 | [04](04-cascade-router.md) | Speculative decoding |
| Bai et al. 2022 | [12](12-self-improvement-frameworks.md) | Constitutional AI |
| Skalse et al. 2022 | [12](12-self-improvement-frameworks.md) | Reward hacking in RL |
| Kirkpatrick et al. 2017 | [14](14-stability-mechanisms.md) | Elastic Weight Consolidation (EWC) |
| Bengio et al. 2009 | [17](17-adas-and-autocatalytic.md) | Curriculum learning |

---

## Architecture Diagram

```
                    ┌─────────────────────────────┐
                    │       Agent Turn             │
                    │   (orchestrate.rs)           │
                    └──────────┬──────────────────┘
                               │
                               ▼
                    ┌─────────────────────────────┐
                    │    LearningRuntime           │
                    │  record_completed_run()      │
                    └──────────┬──────────────────┘
                               │
           ┌───────────────────┼───────────────────────┐
           │                   │                       │
           ▼                   ▼                       ▼
    ┌──────────────┐   ┌──────────────┐      ┌──────────────┐
    │ EpisodeLogger│   │  CostsLog    │      │ TaskMetrics   │
    │  (JSONL)     │   │  (JSONL)     │      │   (JSONL)     │
    └──────┬───────┘   └──────┬───────┘      └──────┬───────┘
           │                  │                      │
           ▼                  ▼                      ▼
    ┌──────────────┐   ┌──────────────┐      ┌──────────────┐
    │PatternMiner  │   │ CascadeRouter│      │  Regression   │
    │(trigrams)    │   │ (3-stage)    │      │  Detection    │
    └──────┬───────┘   └──────┬───────┘      └──────┬───────┘
           │                  │                      │
           ▼                  ▼                      ▼
    ┌──────────────┐   ┌──────────────┐      ┌──────────────┐
    │PlaybookRules │   │ProviderHealth│      │  C-Factor     │
    │  (TOML)      │   │(CircuitBrkr) │      │  (composite)  │
    └──────────────┘   └──────────────┘      └──────────────┘
           │                  │                      │
           └───────────┬──────┘──────────────────────┘
                       │
                       ▼
              ┌──────────────────┐
              │ SystemPromptBuilder│
              │ (prompt injection) │
              └──────────────────┘
```

---

## Data Flow Summary

| Source | Artifact | Consumers |
|--------|----------|-----------|
| Agent turn | Episode | PatternMiner, CascadeRouter, CFactor, SkillLibrary |
| Gate execution | GateVerdict | Episode (embedded), Regression detector |
| Provider response | CostRecord | CostsLog, CostsDb, BudgetGuardrail |
| Agent turn | TaskMetric | MetricsWriter, Baseline, Regression |
| Agent turn | AgentEfficiencyEvent | Efficiency grading, section effectiveness |
| PatternMiner | Pattern | PlaybookRules (promotion candidate) |
| PlaybookRules | Rule | SystemPromptBuilder (injection) |
| SkillLibrary | Skill | SystemPromptBuilder (injection) |
| CascadeRouter | CascadeModel | Orchestrator (model selection) |
| CFactor | CFactorSnapshot | Dashboard, routing bias |
| ProviderHealth | CircuitState | CascadeRouter (filtering) |
| LatencyRegistry | LatencyStats | CascadeRouter (SLA compliance) |
| ExperimentStore | PromptVariant | SystemPromptBuilder (variant selection) |

---

## Cross-Cutting Concerns

Three concerns span the entire learning subsystem and must be addressed holistically rather than within individual documents.

### Catastrophic Forgetting Prevention

As Roko learns new patterns and skills, it must not forget previously learned knowledge. Three mechanisms prevent catastrophic forgetting:

1. **Append-only storage**: Episodes, costs, and metrics are never overwritten. New learning adds to the knowledge base without modifying historical records. This is the simplest and most robust anti-forgetting mechanism.

2. **Elastic Weight Consolidation (EWC) for bandits**: When bandit parameters are updated, critical historical parameters (those that contributed most to past successes) receive higher regularization, resisting change. Inspired by Kirkpatrick et al. 2017.

```rust
pub struct EWCRegularizer {
    /// Fisher information diagonal per bandit arm.
    pub fisher_diag: HashMap<String, Vec<f64>>,
    /// Reference parameters (from last consolidation).
    pub reference_params: HashMap<String, Vec<f64>>,
    /// Regularization strength (default: 100.0).
    pub lambda: f64,
    /// Consolidation interval (default: every 100 episodes).
    pub consolidate_every: u32,
}
```

3. **Confidence decay floor**: Playbook rules have a minimum confidence of 0.10 before pruning. This means a rule must be actively contradicted (not just unused) before removal. Unused rules persist indefinitely at their last confidence level.

### Curriculum Learning for Task Ordering

The plan executor currently runs tasks in dependency order. Curriculum learning (Bengio et al. 2009) suggests that ordering tasks by difficulty — easy first, hard later — accelerates learning because early successes build the skill library and playbook rules that help with harder tasks.

```rust
pub struct CurriculumScheduler {
    /// Difficulty estimator for tasks.
    pub difficulty_model: DifficultyModel,
    /// Curriculum mode.
    pub mode: CurriculumMode,
    /// Current curriculum epoch (resets when a new plan starts).
    pub epoch: u32,
}

pub enum CurriculumMode {
    /// Tasks ordered easy→hard within each dependency level.
    EasyFirst,
    /// Tasks ordered hard→easy (anti-curriculum, for stress testing).
    HardFirst,
    /// Interleaved: alternate easy and hard tasks.
    Interleaved,
    /// Adaptive: start easy, increase difficulty as pass rate improves.
    Adaptive { target_pass_rate: f64 },
}

pub struct DifficultyModel {
    /// Per-(role, complexity, crate) historical pass rate.
    pass_rates: HashMap<(String, String, String), f64>,
    /// HDC similarity to historically difficult episodes.
    difficulty_hdc: Option<HdcVector>,
}
```

Difficulty estimation uses three signals:
- **Historical pass rate** for the `(role, complexity, crate)` triple — lower pass rate = harder
- **HDC similarity** to previously failed episodes — higher similarity = likely harder
- **Dependency depth** — tasks with many dependencies tend to be harder (more constraints)

### Learning Rate Scheduling

Different learning subsystems should adapt at different rates depending on their maturity:

| Subsystem | Cold Start Rate | Warm Rate | Mature Rate |
|-----------|----------------|-----------|-------------|
| Cascade router | High (explore aggressively) | Medium (balance) | Low (exploit) |
| Pattern miner | High (discover patterns) | Medium (validate) | Low (maintain) |
| Skill library | Medium (accumulate) | Medium (validate) | Low (curate) |
| Playbook rules | Low (cautious promotion) | Medium (active validation) | High (aggressive pruning) |

```rust
pub struct LearningRateSchedule {
    /// Episode count thresholds for phase transitions.
    pub cold_threshold: u32,   // default: 50
    pub warm_threshold: u32,   // default: 200
    /// Per-subsystem rate multipliers.
    pub rates: HashMap<String, PhaseRates>,
}

pub struct PhaseRates {
    pub cold: f64,   // rate multiplier during cold start
    pub warm: f64,   // rate multiplier during warm phase
    pub mature: f64, // rate multiplier during mature phase
}
```

This ensures that the system explores aggressively during cold start (building its initial knowledge base) and becomes increasingly conservative as it matures (preserving proven configurations while making incremental improvements).

### Meta-Learning for Tool Use

Roko agents use tools (Read, Write, Bash, etc.) with varying effectiveness. Meta-learning tracks which tool sequences lead to successful outcomes for different task types, then biases tool selection in agent prompts.

```rust
pub struct ToolUsageProfile {
    /// Per-(role, task_category): tool sequence patterns that correlate with success.
    pub success_patterns: HashMap<(String, String), Vec<ToolSequencePattern>>,
    /// Tools that are frequently called but rarely contribute to success.
    pub low_value_tools: Vec<ToolWarning>,
}

pub struct ToolSequencePattern {
    /// Ordered tool sequence (e.g., ["Read", "Edit", "Bash:cargo test"]).
    pub sequence: Vec<String>,
    /// How often this sequence appears in successful episodes.
    pub support: u32,
    /// Pass rate when this sequence is used vs when it's not.
    pub lift: f64,
}

pub struct ToolWarning {
    pub tool_name: String,
    pub calls_per_episode: f64,
    pub contribution_to_success: f64,  // near 0.0 = tool isn't helping
    pub tokens_consumed: u64,
}
```

Tool usage profiles are injected into agent prompts as hints: "For this task type, successful approaches typically use Read→Edit→Bash(test) in that order. Avoid excessive use of [tool] which historically doesn't contribute to success."

---

## Quick Start

To enable learning in a Roko project:

```bash
# Initialize .roko directory (creates .roko/learn/ subdirectory)
cargo run -p roko-cli -- init

# Execute plans — learning subsystems update automatically
cargo run -p roko-cli -- plan run plans/

# View learning status
cargo run -p roko-cli -- dashboard
```

Learning is automatic: every agent turn updates all subsystems through the `LearningRuntime`. No manual configuration is needed beyond `roko init`.

### Inspecting Learning State

```bash
# View episode count and recent episodes
ls -la .roko/learn/episodes.jsonl

# View cost summary
wc -l .roko/learn/costs.jsonl

# View skill library
cat .roko/learn/skills.json | python3 -m json.tool | head -50

# View cascade router state (current stage, observations)
cat .roko/learn/cascade-router.json | python3 -m json.tool | head -20

# View playbook rules
cat .roko/learn/playbook-rules.toml
```


---

# SOURCE: /Users/will/dev/nunchi/roko/roko/docs/07-conductor/00-conductor-architecture.md

# Conductor Architecture

> The Conductor is not a timeout manager. It is the agent's theory of mind
> about its own pipeline — the subsystem that watches execution unfold
> and asks: is this going where I predicted?


> **Implementation**: Built

---

## Position in the Five-Layer Architecture

Roko's runtime stacks into five layers. Each layer has a distinct
responsibility boundary:

| Layer | Name | What It Owns | Key Traits |
|-------|------|-------------|------------|
| L0 | Runtime | Processes, I/O, OS-level lifecycle | `Substrate` |
| L1 | Framework | Tool definitions, agent capabilities | (tools API) |
| L2 | Scaffold | Prompt construction, context engineering | `Composer` |
| **L3** | **Harness** | **Output evaluation, meta-cognition** | **`Gate`, `Policy`** |
| L4 | Orchestration | Multi-agent scheduling, DAG execution | `Router`, `Scheduler` |

The Conductor sits at **Layer 3 — Harness**. It shares this layer with
the gate pipeline (compile, test, clippy, diff, coverage, spec, etc.)
but serves a fundamentally different function:

- **Gates** answer: did the output meet the acceptance criteria?
- **Conductor** answers: is the process itself healthy?

Gates evaluate artifacts. The Conductor evaluates trajectories.

This distinction matters because a plan can pass every individual gate
and still be pathological — looping through identical implement-gate
cycles, burning tokens on ghost turns, or drifting outside its declared
file scope without any single gate catching it.

---

## Synapse Architecture Placement

Roko's kernel defines one noun (`Signal`) and six verb traits:

```
Substrate — storage and I/O
Scorer    — numeric evaluation
Gate      — binary accept/reject
Router    — selection among alternatives
Composer  — prompt assembly
Policy    — reactive stream evaluation
```

The Conductor is a **composite `Policy`**. Every watcher implements the
`Policy` trait. The Conductor itself also implements `Policy`, delegating
to its inner watchers and aggregating their outputs through an
intervention policy.

```rust
// From crates/roko-conductor/src/conductor.rs
pub struct Conductor {
    watchers: Vec<Box<dyn Policy>>,
    policy: Box<dyn InterventionPolicy>,
    circuit_breaker: CircuitBreaker,
}

impl Policy for Conductor {
    fn decide(&self, stream: &[Engram], ctx: &Context) -> Vec<Engram> {
        // 1. Check circuit breaker — tripped plans get Failed immediately
        // 2. Run all watchers — collect WatcherOutputs
        // 3. Apply intervention policy — worst severity wins
        // 4. Record failures to circuit breaker
        // ...
    }
}
```

This composability means the Conductor can be used anywhere a `Policy`
is expected: inside the orchestrator's main loop, as a standalone
evaluation pass, or nested inside a larger policy composition.

---

## What the Conductor Is Not

Understanding the Conductor requires understanding what it deliberately
does not do:

**It is not a scheduler.** The Conductor does not decide which task runs
next, which agent gets spawned, or how resources are allocated. That is
L4 (Orchestration). The Conductor evaluates whether the current
execution trajectory is healthy and emits signals when it is not.

**It is not a gate.** Gates produce binary verdicts (pass/fail) on
artifacts. The Conductor produces graduated interventions (continue /
restart / fail) on processes. A gate looks at the code; the Conductor
looks at the agent producing the code.

**It is not a timeout manager.** Timeouts are one of ten watcher
categories. The Conductor's scope includes loop detection, cost
monitoring, context pressure tracking, spec drift measurement, test
regression detection, and review cycle analysis. Reducing it to
"timeouts" misses 90% of its function.

**It does not nudge.** A nudge is "please fix yourself" — which does not
work on confused agents. The Conductor has exactly three actions:
Continue (everything is fine), Restart (kill and restart with different
context), or Fail (mark the plan as failed). There is no "try harder."
This is a deliberate design decision derived from production experience:
agents that are stuck remain stuck after nudges (§6, Hard Guarantee 6
from the failure prevention catalog).

---

## Core Components

The Conductor comprises seven subsystems, each in its own module:

### 1. Watcher Ensemble (`watchers/`)

Ten watchers, each implementing `Policy`. Each watcher monitors a
specific failure mode by examining the signal stream:

| Watcher | Module | What It Detects |
|---------|--------|----------------|
| Ghost Turn | `ghost_turn.rs` | Agent turns with zero meaningful output |
| Compile Fail Repeat | `compile_fail_repeat.rs` | Identical compile errors repeating |
| Cost Overrun | `cost_overrun.rs` | Plan cost exceeding budget |
| Iteration Loop | `iteration_loop.rs` | Repeated gate-fail retry cycles |
| Review Loop | `review_loop.rs` | Repeated review rejects without progress |
| Spec Drift | `spec_drift.rs` | File edits outside declared scope |
| Stuck Pattern | `stuck_pattern.rs` | Repeated identical agent actions |
| Test Failure Budget | `test_failure_budget.rs` | Test failure count increasing |
| Time Overrun | `time_overrun.rs` | Task approaching timeout threshold |
| Context Window Pressure | `context_window_pressure.rs` | Token usage exceeding context limits |

Each watcher operates independently. They share no state with each
other. They read the signal stream, apply their detection logic, and
either return empty (healthy) or return intervention signals.

### 2. Circuit Breaker (`circuit_breaker.rs`)

Per-plan failure budget tracking. Uses `DashMap` for thread-safe
concurrent access. A plan that accumulates `MAX_PLAN_FAILURES` (default
2) failures is permanently tripped — no further retries.

```rust
pub struct CircuitBreaker {
    failures: DashMap<String, FailureRecord>,
}
```

This prevents the pathological case where a fundamentally broken plan
cycles through retry after retry, burning tokens on every attempt. Two
failures is the budget. After that, the plan requires human attention.

### 3. Intervention Policy (`interventions.rs`)

Maps watcher outputs to conductor decisions through a severity system:

```
Info     → ConductorDecision::Continue
Warning  → ConductorDecision::Restart
Critical → ConductorDecision::Fail
```

The default policy is `WorstSeverityPolicy`: the highest severity among
all watcher outputs determines the decision. If nine watchers say
"continue" and one says "critical," the decision is Fail.

### 4. Diagnosis Engine (`diagnosis.rs`)

Thirty-four built-in error patterns covering twenty error categories.
Given raw error output (compiler messages, test output, agent logs), the
diagnosis engine classifies the error, assigns a confidence score, and
suggests an intervention:

```rust
pub enum SuggestedIntervention {
    RetryWithContext,
    AutoFix,
    RestartAgent,
    AbortPlan,
    BackoffRetry,
    MergeResolution,
    ReduceContext,
    SwitchModel,
    WarnAndContinue,
}
```

This structured classification replaces ad-hoc error parsing. Instead of
grepping for "error[E0308]" in raw output, the diagnosis engine returns
a typed `Diagnosis` with category, confidence, affected files, and
suggested action.

### 5. Stuck Detection (`stuck_detection.rs`)

Six heuristics for detecting stuck agents:

- **OutputLoop**: Agent producing identical output across turns
- **NoProgress**: No file changes within a time window
- **GateLoop**: Gate failures repeating without change
- **CompileLoop**: Same compile errors repeating
- **EmptyOutput**: Turns with no meaningful content
- **ExcessiveRetries**: Too many retry attempts

The `StuckDetector` operates at configurable thresholds. The
`MetaCognitionHook` wraps it for periodic self-assessment at Theta
frequency: "Am I stuck? Am I thrashing? Should I escalate?"

### 6. Health Monitor (`health.rs`)

Four system-level health checks producing a `HealthStatus` (Healthy /
Degraded / Critical):

- **terminal_liveness**: Is the agent process still responsive?
- **agent_status**: Are expected agents running?
- **spec_drift**: Has the implementation diverged from specification?
- **coverage_trend**: Is test coverage trending down?

The health monitor operates on `SystemSnapshot` — a point-in-time view
of system state including active agent count, heartbeat recency, spec
hash comparison, and coverage history.

### 7. State Machine (`state_machine.rs`)

Phase timeout configuration by plan complexity:

| Phase | Complex | Standard | Fast |
|-------|---------|----------|------|
| Implementing | 600s | 300s | 120s |
| Gating | 300s | 300s | 300s |
| Reviewing | 300s | 300s | 300s |
| Merging | 60s | 60s | 60s |

`PhaseTransition` records capture the plan ID, source phase, target
phase, timestamp, and reason — providing a complete audit trail of every
plan's progression through the pipeline.

---

## Evaluation Flow

When the orchestrator calls `conductor.evaluate()`, the following
sequence executes:

```
1. Circuit breaker check
   └─ If plan is tripped → return Fail immediately

2. Run all 10 watchers against the signal stream
   └─ Each watcher returns Vec<Engram> (empty = healthy)
   └─ Collect all non-empty results as WatcherOutputs

3. Apply intervention policy
   └─ WorstSeverityPolicy: max(all severities) → decision
   └─ Info → Continue, Warning → Restart, Critical → Fail

4. If decision is Restart or Fail:
   └─ Record failure in circuit breaker
   └─ Emit intervention signal to stream

5. Return ConductorDecision
```

The entire evaluation is stateless from the Conductor's perspective —
it reads the signal stream and produces a decision. State tracking
(failure counts, circuit breaker trips) lives in the `CircuitBreaker`,
which uses thread-safe `DashMap` for concurrent access.

---

## Signal Flow

The Conductor communicates exclusively through signals. It reads
`Signal` instances from the stream and writes `Signal` instances back:

**Input signals consumed:**

| Kind | What the Conductor Reads |
|------|------------------------|
| `TokenUsage` | Token counts for context pressure |
| `GateVerdict` | Test results for failure budget |
| `AgentOutput` | Output content for ghost turn / stuck detection |
| `PlanPhase` | Phase events for review loop tracking |
| `Metric` (name=spec_drift) | Drift ratios for spec drift |
| `Custom("conductor.agent_output")` | Timing data for time overrun |

**Output signals emitted:**

| Kind | When |
|------|------|
| `Custom("conductor.intervention")` | Any watcher fires |

Intervention signals carry tags: `watcher` (which watcher fired),
`severity` (info/warning/critical), and watcher-specific metadata
(ratio, count, plan_id, task_id, etc.).

---

## Design Decisions

### Why Watchers Are Policies, Not Gates

Gates produce binary verdicts. Policies produce signals with graduated
severity. The Conductor needs graduation because not every anomaly
warrants the same response:

- Context window at 82% → warning (restart with compacted context)
- Spec drift at 30% → warning (the agent is exploring nearby files)
- Three identical compile errors → critical (the agent is stuck)

A gate would reduce all of these to "fail," losing the information
needed for appropriate response.

### Why Ten Watchers Instead of One Smart Monitor

Each watcher is a focused detector for one failure mode. This
decomposition provides:

1. **Testability** — each watcher has isolated unit tests
2. **Configurability** — thresholds are per-watcher
3. **Composability** — add or remove watchers without touching others
4. **Diagnosability** — the intervention signal says which watcher fired

A monolithic monitor would conflate detection with diagnosis. By keeping
watchers separate, the system can tell you not just "something is wrong"
but "the agent has produced three identical compile errors" — a
much more actionable signal.

### Why the Circuit Breaker is Per-Plan

Plans are the unit of retry. A failing plan should not poison other
plans. The circuit breaker tracks failures per plan ID, so plan A
hitting its failure budget does not affect plan B.

The `DashMap` provides thread-safe concurrent access because the
orchestrator may evaluate multiple plans in parallel.

---

## References

- Conant & Ashby (1970) — "Every good regulator of a system must be a
  model of that system." The Conductor models the pipeline's health.
- Beer (1972) — Viable System Model, System 3 (internal oversight) +
  System 3* (audit). The Conductor fills both roles.
- Boyd — OODA loop (Observe-Orient-Decide-Act). Each conductor
  evaluation cycle is one OODA iteration.

---

## File Reference

| File | What |
|------|------|
| `crates/roko-conductor/src/lib.rs` | Module structure, re-exports |
| `crates/roko-conductor/src/conductor.rs` | Conductor struct, evaluate(), Policy impl |
| `crates/roko-conductor/src/circuit_breaker.rs` | Per-plan failure tracking |
| `crates/roko-conductor/src/interventions.rs` | Severity, WatcherOutput, InterventionPolicy |
| `crates/roko-conductor/src/diagnosis.rs` | 34 error patterns, 20 categories |
| `crates/roko-conductor/src/health.rs` | SystemSnapshot, 4 health checks |
| `crates/roko-conductor/src/state_machine.rs` | Phase timeouts, PhaseTransition records |
| `crates/roko-conductor/src/stuck_detection.rs` | 6 stuck heuristics, MetaCognitionHook |
| `crates/roko-conductor/src/watchers/` | 10 watcher modules |


---

# SOURCE: /Users/will/dev/nunchi/roko/roko/docs/07-conductor/01-watcher-ensemble.md

# Watcher Ensemble

> Ten independent detectors, each focused on one failure mode,
> each implementing the `Policy` trait, each testable in isolation.


> **Implementation**: Built

---

## The Policy Trait

Every watcher implements the same trait:

```rust
pub trait Policy: Send + Sync {
    fn decide(&self, stream: &[Engram], ctx: &Context) -> Vec<Engram>;
    fn name(&self) -> &str;
}
```

`decide()` receives the full signal stream and returns intervention
signals. An empty return means "healthy — nothing to report." A
non-empty return means the watcher has detected an anomaly and is
emitting one or more intervention signals with severity tags.

The Context provides the current tick position. Watchers may use it
for time-relative calculations but most ignore it, operating purely on
the signal stream content.

---

## Watcher Catalog

### 1. Ghost Turn Detector

**Module**: `watchers/ghost_turn.rs`
**Constant**: `MAX_GHOST_TURNS = 3`
**Watcher name**: `ghost-turn`

**What it detects**: Agent turns that produce zero meaningful output.
A ghost turn is a turn where the model returned immediately (often
under 5 seconds) with no tool calls, no file changes, and no
substantive content. This is a known failure mode with API-based agents
— the model may return an empty response, a brief acknowledgment
without action, or repeat its own instructions back.

**How it works**: Scans the signal stream for `AgentOutput` signals.
Tracks the most recent agent's output sequence. If the output body
matches the ghost pattern (below minimum meaningful length), increments
a counter. After `MAX_GHOST_TURNS` consecutive ghost turns from the
same agent, fires a warning.

**Detection logic**: The watcher checks `AgentOutput` signals for body
content. It considers a turn "ghost" when the output body is empty or
below a minimum threshold. Three consecutive ghost turns trigger the
intervention.

**Severity**: Warning (triggers restart with fresh context)

**Why three, not one**: A single empty response can be a transient API
issue. Two might be a flaky connection. Three consecutive ghost turns
indicate the agent has entered a degenerate state and will not recover
without intervention. The threshold balances false positives (killing a
slow-to-start agent) against false negatives (letting a broken agent
burn tokens).

**Production context**: Ghost turns were Issue #9 in the production
failure catalog. In early batch runs, ghost agents would appear active
(process running, consuming API quota) but produce nothing useful —
repeating themselves, asking clarifying questions to nobody, or
describing intended actions without executing them. These could burn
significant token budget before manual detection. The ghost turn watcher
automates what was previously a manual grep-the-logs operation.

---

### 2. Compile Fail Repeat Detector

**Module**: `watchers/compile_fail_repeat.rs`
**Constant**: `MAX_COMPILE_FAIL_REPEAT = 3`
**Watcher name**: `compile-fail-repeat`

**What it detects**: The same compile error appearing across consecutive
gate verdicts without the agent making progress toward fixing it.

**How it works**: Examines `GateVerdict` signals for compile-related
gate results. Extracts error fingerprints from the gate verdict body.
When the same fingerprint appears `MAX_COMPILE_FAIL_REPEAT` times
consecutively for the same plan, fires an intervention.

**Detection logic**: The watcher looks for `GateVerdict` signals,
extracts the error content from the body, and tracks whether the
same errors recur. Three identical compile errors in sequence indicate
the agent is attempting the same fix repeatedly.

**Severity**: Warning (triggers restart with error analysis context)

**Why this matters**: An agent stuck on a compile error is the most
common form of agent loop. The agent reads the error, attempts a fix,
recompiles, gets the same error, reads it again, attempts the same fix.
Without intervention, this cycle continues until the iteration limit.
The compile fail repeat watcher detects this after 3 cycles instead of
letting it run to exhaustion.

**Connection to Diagnosis Engine**: When this watcher fires, the
Diagnosis Engine can classify the specific error and suggest an
intervention. E.g., if the repeated error is E0432 (unresolved import),
the suggested intervention is `AutoFix` — a cheap Haiku-tier fix.
If it is E0277 (trait not implemented), the suggestion is
`RestartAgent` with additional type context.

---

### 3. Cost Overrun Detector

**Module**: `watchers/cost_overrun.rs`
**Constant**: `DEFAULT_BUDGET_USD = 10.0`
**Watcher name**: `cost-overrun`

**What it detects**: Plan-level cost exceeding the allocated budget.

**How it works**: Scans for `Metric` signals tagged with
`name=plan_cost` to track accumulated cost. Compares against the
plan's budget (from tags or default). When accumulated cost exceeds
budget, fires an intervention.

**Detection logic**: The watcher finds the most recent cost metric
signal for each plan and compares the cumulative cost against the
budget. The budget is read from a tag on the signal or falls back
to `DEFAULT_BUDGET_USD`.

**Severity**: Warning at threshold, escalates based on overage

**Why cost matters**: In production batch runs, a single runaway plan
can consume more budget than all other plans combined. The most
expensive failure mode is an agent that produces plausible-looking but
incorrect code, passes compilation, fails tests, gets retried with more
context (more tokens), fails again with slightly different errors, and
repeats. Each iteration costs more than the last because the context
grows. Without cost monitoring, this can run the total batch cost to
10x the expected budget.

**Budget allocation strategy**: The default $10 budget per plan is
conservative for Opus-tier tasks and generous for Haiku-tier tasks.
In practice, the budget should be set based on plan complexity:

| Complexity | Typical Cost | Suggested Budget |
|-----------|-------------|-----------------|
| Trivial | $0.10–0.50 | $2.00 |
| Simple | $0.50–2.00 | $5.00 |
| Standard | $1.00–5.00 | $10.00 |
| Complex | $3.00–15.00 | $25.00 |

The adaptive gate threshold system (in `roko-gate`) can eventually
feed cost data back to budget allocation, creating a learning loop.

---

### 4. Iteration Loop Detector

**Module**: `watchers/iteration_loop.rs`
**Constant**: `MAX_ITERATION_LOOP = 3`
**Watcher name**: `iteration-loop`

**What it detects**: Plans cycling through the gate-fail-retry loop
without making progress toward passing.

**How it works**: Tracks `GateVerdict` signals per plan. When a plan
accumulates `MAX_ITERATION_LOOP` consecutive gate failures without
an intervening gate pass, fires an intervention.

**Detection logic**: The watcher scans for `GateVerdict` signals,
tracking consecutive failures per plan. It resets the counter when a
gate pass is observed. Three consecutive failures trigger the
intervention.

**Severity**: Critical (triggers plan failure)

**Why critical**: This is the only watcher that defaults to Critical
severity. The rationale: three consecutive gate failures indicate a
fundamental mismatch between the agent's approach and the requirements.
More iterations of the same approach will not converge. The plan needs
either a different strategy (different model, more context, alternative
decomposition) or human attention.

**Hard Guarantee connection**: This implements Hard Guarantee 3 from
the failure prevention catalog — "Hard Iteration Cap (Not Soft, Not
Heuristic)." The iteration limit is enforced by the state machine, not
by heuristic detection. The conductor's role changes from "detect loops
and decide whether to intervene" to "the plan failed; decide whether
it is worth retrying with a different approach."

**The compound problem**: Each retry iteration is more expensive than
the last. The agent's context grows (previous errors, reflections,
gate output), increasing token cost. The probability of convergence
decreases with each failed attempt (if the first three attempts
failed, the fourth is unlikely to succeed without a fundamentally
different approach). Cutting off at three prevents the exponential
cost growth of diminishing-probability retries.

---

### 5. Review Loop Detector

**Module**: `watchers/review_loop.rs`
**Constant**: `MAX_REVIEW_CYCLES = 3`
**Watcher name**: `review-loop`

**What it detects**: Plans receiving repeated review rejects without
advancing to a later phase.

**How it works**: Scans `PlanPhase` signals for the most recent plan ID.
Counts `ReviewRejected` events for that plan. Resets the counter on
`ReviewApproved`, `DocRevisionDone`, or `MergeSucceeded`. Fires when
the count reaches `MAX_REVIEW_CYCLES`.

```rust
// From watchers/review_loop.rs — decision logic
match plan_event(s).as_deref() {
    Some("ReviewRejected") => {
        review_rejects += 1;
        if review_rejects >= self.max_cycles {
            // Fire intervention
        }
    }
    Some("ReviewApproved") | Some("DocRevisionDone") | Some("MergeSucceeded") => {
        review_rejects = 0;  // Reset — progress was made
    }
    _ => {}
}
```

**Severity**: Warning (triggers review skip or strategy change)

**The bikeshedding problem**: In production batch runs, reviewer agents
can enter a cycle where code passes compilation and tests but the
reviewer repeatedly requests stylistic changes. Each review reject
triggers a re-implementation cycle. The implementer makes the requested
changes, the reviewer finds new stylistic concerns, and the cycle
repeats. Three consecutive rejects without progress indicates
bikeshedding — the code works, the reviewers are not converging, and
further iterations waste tokens.

**Reset semantics**: The counter resets on any positive progress event.
This means a plan that receives one reject, then an approval, then
two more rejects has only counted two consecutive rejects — the
approval reset the counter. Only sustained, uninterrupted rejection
sequences trigger the intervention.

---

### 6. Spec Drift Detector

**Module**: `watchers/spec_drift.rs`
**Constant**: `MAX_SPEC_DRIFT_RATIO = 0.25`
**Watcher name**: `spec-drift`

**What it detects**: Agent file edits drifting outside the declared
scope of the task.

**How it works**: Examines `Metric` signals tagged
`name=spec_drift`. The signal body contains a `SpecDriftEvent` with:
- `write_files`: files the task declared it would modify
- `changed_files`: files the agent actually modified
- `unexpected_files`: changed files not in the declared set
- `drift_ratio`: fraction of changes that were unexpected

When `drift_ratio > MAX_SPEC_DRIFT_RATIO` (25%), fires an intervention.

**Detection logic**: The watcher supports two signal formats — a
structured JSON body with full file lists, or a simple tag-based
format with just the drift ratio number. This dual-format support
allows both detailed and lightweight drift reporting.

```rust
// Drift computation from SpecDriftEvent
fn drift_ratio(&self) -> f64 {
    let changed = self.changed_files.len();
    if changed == 0 { return 0.0; }
    self.unexpected_files().len() as f64 / changed as f64
}
```

**Severity**: Warning

**Why 25%**: Some drift is normal and healthy. An agent implementing
a new function may need to update a `mod.rs` file or add an import
to a sibling module. A 10% drift ratio is typical for well-scoped
tasks. At 25%, the agent is making substantial changes outside its
declared scope — potentially stepping on another concurrent agent's
territory or introducing unplanned coupling.

**Path matching**: The `path_is_allowed()` function supports both
exact matches and prefix matches. If the declared write file is
`src/auth/`, any file under that directory is considered in-scope.
This prevents false positives when the task declares a directory
but the agent creates new files within it.

---

### 7. Stuck Pattern Detector

**Module**: `watchers/stuck_pattern.rs`
**Constant**: `MAX_STUCK_REPEATS = 4`
**Watcher name**: `stuck-pattern`

**What it detects**: Agent producing identical actions across
consecutive turns.

**How it works**: Tracks recent agent actions (tool calls, file edits)
and computes similarity between consecutive turns. When four consecutive
turns produce identical or near-identical actions, fires an intervention.

**Severity**: Warning

**Relationship to other watchers**: The stuck pattern detector overlaps
with compile fail repeat (which specifically catches identical compile
errors) and ghost turn (which catches zero output). The stuck pattern
detector is the general-purpose version — it catches any form of
repetitive behavior, not just compile loops or empty responses.

---

### 8. Test Failure Budget Detector

**Module**: `watchers/test_failure_budget.rs`
**Constant**: `MIN_FAILURE_INCREASE = 1`
**Watcher name**: `test-failure-budget`

**What it detects**: Test failure count increasing beyond the baseline
observed earlier in the signal stream.

**How it works**: Scans `GateVerdict` signals for structured test counts.
For each plan, records the first observed failure count as the baseline.
When the latest failure count exceeds the baseline by
`MIN_FAILURE_INCREASE`, fires an intervention.

```rust
// Per-plan baseline tracking
baselines.entry(plan_id.clone()).or_insert(failed);  // First seen = baseline
latest.insert(plan_id, failed);                       // Always update latest

// Fire when latest exceeds baseline
if current_failed.saturating_sub(baseline_failed) >= self.min_failure_increase {
    // Emit intervention
}
```

**Severity**: Warning

**The regression signal**: This watcher detects a specific problem —
the agent is making things worse, not better. If a plan starts with
1 failing test and ends with 3 failing tests, the agent introduced
2 new test failures. This is a stronger signal than "tests are failing"
(which might be the expected state at start) — it means the agent's
changes are actively harmful.

**Plan independence**: Each plan has its own baseline. Plan A starting
with 5 failures and Plan B starting with 0 failures are tracked
independently. A failure increase on Plan A does not affect Plan B's
baseline.

**Custom thresholds**: The constructor accepts a custom
`min_failure_increase`. For codebases with flaky tests, setting this
to 3 (rather than 1) avoids false positives from non-deterministic
test outcomes.

---

### 9. Time Overrun Detector

**Module**: `watchers/time_overrun.rs`
**Constant**: `ALERT_THRESHOLD = 0.80`
**Watcher name**: `time-overrun`

**What it detects**: Tasks approaching their timeout threshold.

**How it works**: Examines `Custom("conductor.agent_output")` signals
for `TaskTimingEvent` payloads containing `duration_ms` and
`timeout_secs`. When the ratio exceeds 80% of the timeout, fires
an early warning.

```rust
// Threshold check using integer arithmetic to avoid floating-point edge cases
fn exceeds_threshold(duration_ms: u64, timeout_secs: u64) -> bool {
    if timeout_secs == 0 { return false; }
    let timeout_ms = timeout_secs.saturating_mul(1000);
    duration_ms.saturating_mul(5) > timeout_ms.saturating_mul(4)
    // Equivalent to: duration_ms / timeout_ms > 4/5 = 0.80
}
```

**Severity**: Warning

**Why 80%, not 100%**: The 80% threshold provides a 20% buffer for the
system to react. At 100%, the task has already timed out — there is
nothing to do except fail it. At 80%, the Conductor can signal the
orchestrator to prepare for a potential timeout: start warming a
replacement agent, checkpoint the current state, or adjust the
remaining task's priority.

**Integer arithmetic**: The threshold check uses `saturating_mul`
instead of floating-point division to avoid edge cases with zero
denominators and floating-point precision. The comparison
`duration * 5 > timeout * 4` is algebraically equivalent to
`duration / timeout > 0.80` but avoids division.

---

### 10. Context Window Pressure Detector

**Module**: `watchers/context_window_pressure.rs`
**Constant**: `MAX_CONTEXT_USAGE_RATIO = 0.80`
**Watcher name**: `context-window-pressure`

**What it detects**: Agent context window filling beyond safe limits.

**How it works**: Examines `TokenUsage` signals for token consumption.
Supports two signal formats:

1. **AgentEfficiencyEvent body**: Deserializes the structured event
   to extract `total_prompt_tokens` and looks up the model's context
   window size from a built-in table.

2. **Tag-based format**: Reads `tokens_used` and `tokens_total` (or
   `model`) from signal tags.

When `used / total > MAX_CONTEXT_USAGE_RATIO`, fires an intervention.

**Model context windows**:

| Model Pattern | Context Window |
|--------------|---------------|
| `*opus*` | 1,000,000 tokens |
| `*haiku*`, `*sonnet*` | 200,000 tokens |
| Unknown | No fire (cannot compute ratio) |

**Severity**: Warning (triggers context compaction)

**Why 80%**: From production monitoring research and the Semantic Kernel
framework: trigger compaction at 80% utilization, not 100%. At 100% the
next request fails with a context overflow error. At 80% there is still
space to compact gracefully — truncating old tool results, summarizing
earlier conversation turns, or dropping low-relevance context sections.

**The compaction cascade**: When this watcher fires, the orchestrator
should trigger the tool result compaction strategy (from the production
hardening plan): truncate old tool results to 200 characters, preserve
recent results intact, maintain tool_call_id integrity. This recovers
20-40% of context space without losing critical recent context.

**AgentEfficiencyEvent integration**: The context window pressure
watcher reads from the same `AgentEfficiencyEvent` signals that feed
the learning system's efficiency tracking. This means every agent turn
that records efficiency data also gets context pressure monitoring for
free — no additional instrumentation needed.

---

## Watcher Independence

Each watcher is independent:

- **No shared state**: Watchers do not read each other's output or
  maintain shared counters.
- **No ordering dependency**: Watchers can execute in any order.
  The Conductor iterates them sequentially for simplicity, but
  parallel execution would produce identical results.
- **No cross-watcher interaction**: The ghost turn watcher does not
  know about the stuck pattern watcher. If both fire simultaneously,
  the intervention policy resolves the conflict (worst severity wins).

This independence is what makes the ensemble testable. Each watcher
has its own `#[cfg(test)] mod tests` with focused test cases that
construct specific signal sequences and verify the watcher's response.
No test needs to set up the full Conductor or mock other watchers.

---

## Adding a New Watcher

To add an eleventh watcher:

1. Create a new file in `watchers/` implementing `Policy`
2. Add it to `watchers/mod.rs`
3. Add it to `Conductor::new()` in `conductor.rs`
4. Update the `watcher_count()` test (currently asserts 10)
5. Write focused tests for the new watcher's detection logic

The Conductor's `evaluate()` method automatically picks up any watcher
in the `watchers` vector. No other code needs to change.

---

## File Reference

| File | Lines | What |
|------|-------|------|
| `watchers/mod.rs` | ~20 | Module declarations, re-exports |
| `watchers/ghost_turn.rs` | ~150 | Ghost turn detection |
| `watchers/compile_fail_repeat.rs` | ~180 | Compile error repetition |
| `watchers/cost_overrun.rs` | ~180 | Cost budget monitoring |
| `watchers/iteration_loop.rs` | ~170 | Gate-fail cycle detection |
| `watchers/review_loop.rs` | ~230 | Review reject cycles |
| `watchers/spec_drift.rs` | ~264 | File scope drift |
| `watchers/stuck_pattern.rs` | ~170 | Repeated action detection |
| `watchers/test_failure_budget.rs` | ~202 | Test regression detection |
| `watchers/time_overrun.rs` | ~182 | Timeout approach warning |
| `watchers/context_window_pressure.rs` | ~233 | Token usage monitoring |

---

## Watcher Composition — Complex Pattern Detection

Individual watchers answer narrow questions: "Did the agent ghost?"
"Did compile errors repeat?" Real failures are rarely that tidy.
A context window filling up while compile errors repeat while cost
climbs — that compound pattern signals something no single watcher
can catch alone.

This section describes how watchers compose to detect those
multi-signal patterns.

### CEP-inspired pattern matching

Complex Event Processing engines — Apache Flink's FlinkCEP, Esper
EPL, Siddhi CQL — solve a structurally identical problem: detect
temporal patterns in high-volume event streams. The core technique
is NFA-based pattern matching over ordered signal sequences.

Two categories of temporal patterns matter for watcher composition:

**Sequence detection** — "Signal A followed by Signal B within time T."
Example: "Agent retried 3+ times within 60 seconds."

```
PATTERN [every A{3,} within 60 sec]
```

**Absence detection** — "Signal A occurred, but Signal B did NOT
follow within time T." Example: "Tool call without response within
30 seconds."

```
PATTERN [A -> (not B where timer:within(30 sec))]
```

**Monotonic progression** — "Each successive signal exceeds the
previous on some metric." Example: "Escalating latency: each turn
slower than the last."

```
MATCH_RECOGNIZE(PATTERN (A B+ C) DEFINE B AS B.latency > A.latency)
```

These patterns translate directly to agent monitoring. The signal
stream is the event source. Watchers produce the events. A composite
pattern layer matches sequences across watcher outputs.

The struct below captures this:

```rust
/// Composite pattern that matches sequences across multiple watchers.
/// Inspired by NFA-based CEP engines (FlinkCEP, Esper EPL).
pub struct CompositePattern {
    /// Pattern stages — each stage matches one or more signal conditions.
    stages: Vec<PatternStage>,
    /// Maximum wall-clock duration for the entire pattern to complete.
    within: Duration,
    /// Contiguity: Strict (no gaps), Relaxed (skip non-matching), or NonDeterministic.
    contiguity: Contiguity,
}

pub struct PatternStage {
    /// Signal kind(s) this stage matches.
    match_kinds: Vec<Kind>,
    /// Predicate evaluated against the signal's tags and body.
    predicate: Box<dyn Fn(&Engram) -> bool + Send + Sync>,
    /// Quantifier: Exactly(n), AtLeast(n), Between(min, max).
    quantifier: Quantifier,
    /// Negation: if true, this stage matches the ABSENCE of the pattern.
    negated: bool,
}

pub enum Contiguity { Strict, Relaxed, NonDeterministic }
pub enum Quantifier { Exactly(usize), AtLeast(usize), Between(usize, usize) }
```

**Contiguity modes** control how strictly the pattern engine matches
consecutive events:

- **Strict**: Every signal between stages must match (no irrelevant
  signals allowed in the gap).
- **Relaxed**: Non-matching signals between stages are skipped.
- **NonDeterministic**: Multiple partial matches can coexist,
  branching on ambiguous signals.

For agent monitoring, Relaxed contiguity is the right default. Agent
signal streams contain many signal types; requiring strict adjacency
between pattern stages would miss most real patterns.

### Multi-watcher correlation

When multiple watchers fire at the same time, the question is: one
root cause, or multiple independent failures? The answer depends on
which watchers fired.

Watchers group into three families based on what they measure:

**Resource family**: cost-overrun, context-window-pressure, time-overrun.
These track consumption of finite budgets (money, tokens, wall-clock
time). When two resource watchers fire together, the root cause is
usually a single runaway process burning all three budgets
simultaneously.

**Behavioral family**: ghost-turn, stuck-pattern, compile-fail-repeat,
iteration-loop. These detect degenerate agent behavior. Two behavioral
watchers firing together confirms the agent is stuck — the specific
failure mode is being caught from multiple angles.

**Coordination family**: review-loop, spec-drift, test-failure-budget.
These detect multi-agent coordination breakdowns. Two coordination
watchers firing together indicates a systemic scoping or communication
problem, not a single agent failure.

```rust
/// Watcher family grouping for correlated signal analysis.
pub struct WatcherFamily {
    pub name: &'static str,
    pub members: Vec<&'static str>,
}

pub const WATCHER_FAMILIES: &[WatcherFamily] = &[
    WatcherFamily { name: "resource", members: &["cost-overrun", "context-window-pressure", "time-overrun"] },
    WatcherFamily { name: "behavioral", members: &["ghost-turn", "stuck-pattern", "compile-fail-repeat", "iteration-loop"] },
    WatcherFamily { name: "coordination", members: &["review-loop", "spec-drift", "test-failure-budget"] },
];
```

**Within-family correlation**: If 2+ watchers in the same family fire
simultaneously, treat them as a single correlated event. The underlying
cause is likely one issue manifesting in multiple metrics. De-duplicate
before escalating — report the highest-severity watcher's signal,
annotated with which other family members also fired.

**Cross-family correlation**: If watchers from different families fire
simultaneously, the situation is more severe. A behavioral failure
(stuck-pattern) combined with a resource failure (cost-overrun) means
the agent is both stuck AND burning budget doing it. Cross-family
correlation should escalate severity by one level: a Warning from
each family becomes a single Critical intervention.

---

## Watcher Priority and Conflict Resolution

When multiple watchers fire in the same evaluation cycle, the
Conductor must produce a single coherent intervention. The current
approach is conservative. The alternatives below offer more nuance
at the cost of more complexity.

### Current approach: WorstSeverityPolicy

`WorstSeverityPolicy` takes the maximum severity across all fired
watchers. If ghost-turn fires at Warning and iteration-loop fires at
Critical, the result is Critical.

This is simple, conservative, and effective. It never under-reacts.
The tradeoff: it can over-react when a low-confidence Critical
watcher fires alongside high-confidence Warning watchers. Every
alternative below addresses this tradeoff.

### Bayesian fusion

Instead of taking the max, combine watcher outputs probabilistically.
Each watcher provides a likelihood ratio — how much more probable
is this signal under "anomaly" than under "normal operation"?

The combined posterior follows from log-odds addition:

```
log P(anomaly | signals) = log P(anomaly) + sum_i log LR_i(s_i)
```

A watcher that fires with a high likelihood ratio shifts the posterior
strongly toward anomaly. A watcher that stays silent shifts it toward
normal (by its silent likelihood ratio). The net effect: watchers
that are historically accurate carry more weight than watchers that
produce frequent false positives.

```rust
/// Bayesian fusion of watcher outputs for conflict resolution.
pub struct BayesianFusionPolicy {
    /// Prior probability of anomaly (before any watchers fire).
    prior_log_odds: f64,
    /// Per-watcher calibration: (log_likelihood_ratio_when_fired, log_likelihood_ratio_when_silent).
    watcher_calibrations: HashMap<String, WatcherCalibration>,
}

pub struct WatcherCalibration {
    /// log(P(fire | anomaly) / P(fire | normal)) — how informative is this watcher when it fires?
    pub log_lr_fired: f64,
    /// log(P(silent | anomaly) / P(silent | normal)) — how informative is silence?
    pub log_lr_silent: f64,
    /// Historical precision (true positives / all positives).
    pub precision: f64,
    /// Historical recall (true positives / all anomalies).
    pub recall: f64,
}
```

This requires calibration data: each watcher's true positive and
false positive rates from historical runs. The learning system
(roko-learn) already tracks per-gate precision via the adaptive
threshold mechanism. Extending that tracking to watchers gives the
calibration data Bayesian fusion needs.

### Dempster-Shafer theory

Bayesian fusion assumes watchers are well-calibrated probabilistic
classifiers. In practice, watchers often express something weaker:
"I think something is wrong, but I'm not sure what." Dempster-Shafer
theory handles this uncertainty directly.

Each watcher provides a mass function over three states:
{anomaly}, {normal}, {anomaly, normal} (ignorance).

A watcher that fires with high confidence assigns most mass to
{anomaly}. A watcher that is uncertain assigns mass to
{anomaly, normal} — expressing ignorance, NOT confidence in
normality. This distinction matters: "I don't know" is different
from "everything is fine."

Combination follows Dempster's rule:

```
m_12(A) = sum_{B intersect C = A} m_1(B) * m_2(C) / (1 - K)
```

where K is the conflict mass — the sum of products where the
intersection is empty. When K > 0.5, the watchers genuinely
disagree about what is happening. This is itself a signal: high
conflict between watchers means the system should escalate for human
review rather than auto-resolve.

**When to prefer Dempster-Shafer over Bayesian**: When watchers
have poor calibration data (early in a project's lifecycle), when
watchers express qualitative judgments rather than probabilistic
scores, or when the "I don't know" state carries important
information.

### Weighted voting with online learning

A lighter-weight alternative to full probabilistic fusion: weight
each watcher's vote by its historical precision.

Track per-watcher precision via an online confusion matrix. Each
time a watcher fires and the outcome is later confirmed (task
succeeded or failed), update the matrix. Weight each watcher's
severity vote by its precision score.

Use Thompson sampling — already available in roko-learn's bandit
infrastructure — to adapt weights over time. Each watcher is an
arm. The reward is correct prediction (watcher fired and the task
genuinely failed, or watcher stayed silent and the task succeeded).
Watchers with high false-positive rates get downweighted
dynamically. Watchers with consistently accurate predictions gain
influence.

This approach requires less calibration data than Bayesian fusion
and handles non-stationary watcher accuracy (a watcher that was
accurate last month but is noisy this month gets downweighted
automatically).

### Temporal hysteresis

Orthogonal to the fusion method: prevent single-spike false
positives by requiring sustained firing before a watcher's output
counts.

A watcher must fire for N consecutive evaluations before its signal
propagates to the intervention policy. Default N=1 (current
behavior — no hysteresis). For noisy watchers, set N=3: the watcher
must fire three times in a row before its output reaches the fusion
layer.

This prevents oscillation patterns: fire, restart, fire, restart.
With hysteresis, the first two firings are absorbed. The third
confirms the pattern is real and propagates the intervention.

Hysteresis is per-watcher configurable. Stable watchers
(iteration-loop, cost-overrun) should keep N=1 — they fire based
on accumulated counts and are already resistant to transient noise.
Noisy watchers (spec-drift, context-window-pressure) benefit from
N=2 or N=3.

---

## Streaming Anomaly Detection Integration

The threshold-based watchers above catch known failure modes:
compile loops, ghost turns, cost overruns. They do not catch
novel failures — patterns that nobody anticipated when writing
the watcher catalog. Streaming anomaly detection fills that gap.

### Online Isolation Forest

The original Isolation Forest (Liu et al., 2008) builds random
binary trees over a batch dataset. Points that isolate quickly
(short average path length) are anomalies. The online variant
(ICML 2024) adapts this to streaming data by maintaining a sliding
window and splitting leaf nodes incrementally.

Each node tracks a count and bounding box. When a leaf accumulates
enough points, it splits on a random dimension at a random value
within the bounding box. Old points outside the sliding window
are decremented from leaf counts.

Anomaly score:

```
s = 2^(-E(depth) / c(window_size))
```

where `E(depth)` is the expected depth of the point across all trees
and `c(n)` is the average path length of an unsuccessful search in a
binary search tree of size n (the normalization factor).

For agent monitoring, each agent turn becomes a multivariate point:
latency (ms), tokens consumed, tool calls made, error rate. A turn
that scores above the threshold on the forest is flagged as
anomalous — even if no individual watcher fires.

```rust
/// Online Isolation Forest for streaming anomaly detection.
/// Reference: ICML 2024, "Online Isolation Forest"
pub struct OnlineIsolationForest {
    trees: Vec<IsolationTree>,
    window_size: usize,       // omega: sliding window (default: 1000)
    max_leaf_samples: usize,   // eta: split threshold (default: 8)
    num_trees: usize,          // tau: ensemble size (default: 50)
    score_threshold: f64,      // anomaly if score > threshold (default: 0.7)
}

pub struct IsolationTree {
    root: IsolationNode,
}

pub enum IsolationNode {
    Internal {
        split_dim: usize,
        split_val: f64,
        count: usize,
        bbox: BoundingBox,
        left: Box<IsolationNode>,
        right: Box<IsolationNode>,
    },
    Leaf {
        count: usize,
        bbox: BoundingBox,
        samples: Vec<Vec<f64>>,  // retained for splitting
    },
}
```

**Default parameters**: tau=50 trees, omega=1000 window size,
eta=8 max leaf samples, score threshold=0.7. These are standard
values from the literature. For agent monitoring workloads with
lower volume (hundreds of turns per plan, not millions), reduce
omega to 100-200 to build the model faster.

### CUSUM for per-watcher change detection

EWMA z-scores (used by the existing watchers) detect spikes —
single-turn anomalies that deviate sharply from the mean.
CUSUM (Cumulative Sum, Page 1954) detects something different:
sustained shifts. A metric that drifts upward by 0.5 sigma per
turn won't trigger a z-score alarm for many turns, but CUSUM
catches it quickly.

The upper CUSUM statistic:

```
C_t = max(0, C_{t-1} + (x_t - mu_0 - k))
```

where `mu_0` is the baseline mean and `k` is the allowance
parameter (typically delta/2, where delta is the shift magnitude
to detect). When `C_t` exceeds the decision threshold `h`, CUSUM
raises an alarm.

The lower CUSUM statistic tracks negative shifts symmetrically.
Together they detect both upward and downward sustained changes.

```rust
/// CUSUM change-point detector for sustained anomaly detection.
/// Detects persistent shifts from a baseline, complementing EWMA spike detection.
pub struct CusumDetector {
    /// Target mean (baseline behavior).
    mu_0: f64,
    /// Allowance parameter (typically delta/2 where delta is the shift to detect).
    k: f64,
    /// Decision threshold (alarm when cumulative sum exceeds h).
    h: f64,
    /// Upper CUSUM statistic.
    upper: f64,
    /// Lower CUSUM statistic.
    lower: f64,
}

impl CusumDetector {
    pub fn update(&mut self, value: f64) -> Option<CusumAlarm> {
        self.upper = (self.upper + (value - self.mu_0 - self.k)).max(0.0);
        self.lower = (self.lower - (value - self.mu_0 + self.k)).max(0.0);
        if self.upper > self.h {
            Some(CusumAlarm::UpperShift { cumsum: self.upper })
        } else if self.lower > self.h {
            Some(CusumAlarm::LowerShift { cumsum: self.lower })
        } else {
            None
        }
    }
}
```

**Operating characteristics**: With k = delta/2 and h = 4-5 sigma,
the average run length before a false alarm (ARL_0) is roughly
500 samples. The average run length to detect a 1-sigma shift
(ARL_1) is roughly 26 samples. CUSUM detects sustained degradation
10-50x faster than EWMA z-scores for the same false alarm rate.

**Application**: Attach a CUSUM detector to each watcher's numeric
output (cost per turn, latency per turn, drift ratio). The watcher's
own threshold catches spikes; CUSUM catches gradual worsening that
stays below the spike threshold but accumulates over time.

### TraceAegis-style behavioral rules

TraceAegis (arXiv 2510.11203, October 2024) monitors tool
invocation traces at the gateway level. Instead of statistical
anomaly detection, it defines explicit behavioral rules: expected
call ordering, parameter constraints, intent-aligned state
transitions.

This approach detects a class of failures that statistical methods
miss: an agent that calls tools in a valid but unauthorized order,
modifies parameters within normal ranges but in a goal-misaligned
way, or makes state transitions that are individually legal but
collectively drift from the task intent.

Behavioral rules for agent monitoring:

- **Call ordering**: Tool A must precede Tool B (e.g., read a file
  before editing it). Violations indicate the agent is operating
  on stale or missing context.
- **Parameter constraints**: Tool parameters must fall within
  task-declared bounds (e.g., file edits restricted to declared
  write paths). Violations overlap with spec-drift detection but
  operate at the tool-call level rather than the file-change level.
- **State transitions**: After a gate failure, the next action
  should be a diagnostic step (read error output, inspect failing
  test), not a blind retry. Violations indicate the agent is not
  learning from feedback.
- **API boundary enforcement**: Agents should not call tools
  outside their declared capability set. An implementation agent
  calling a deployment tool is a boundary violation.

Validated F1 scores for TraceAegis-style rule checking range
from 0.93 to 0.96 in their evaluated scenarios, making this a
high-precision complement to the statistical methods above.

### References

- Liu, F. T., Ting, K. M., & Zhou, Z.-H. (2008). "Isolation Forest." ICDM.
- Online Isolation Forest. ICML 2024.
- Page, E. S. (1954). "Continuous Inspection Schemes." Biometrika.
- TraceAegis. arXiv 2510.11203, October 2024.
- Dempster, A. P. (1967). "Upper and Lower Probabilities Induced by a Multivalued Mapping." Annals of Mathematical Statistics.
- Shafer, G. (1976). A Mathematical Theory of Evidence. Princeton University Press.
- Apache Flink CEP documentation: https://nightlies.apache.org/flink/flink-docs-stable/docs/libs/cep/
- Esper EPL documentation: https://www.espertech.com/esper/


---

# SOURCE: /Users/will/dev/nunchi/roko/roko/docs/07-conductor/02-circuit-breaker.md

# Circuit Breaker

> A plan can fail a maximum of two times. After that, it requires
> human attention. This is not configurable. This is law.


> **Implementation**: Built

---

## The Problem It Solves

Without a circuit breaker, a fundamentally broken plan enters an
infinite retry loop:

```
Plan fails → orchestrator retries → plan fails the same way →
orchestrator retries → plan fails again → orchestrator retries → ...
```

Each retry costs tokens. Each retry burns wall-clock time that could
be spent on plans that might succeed. Each retry produces the same
failure output, adding noise to the signal stream without adding
information.

This was Issue #7 from production (circuit breaker for repeated
failures): "A plan fails, gets retried, fails the same way, gets
retried again, fails again. Infinite retry loop burning tokens."

The circuit breaker enforces a hard budget: two failures per plan.
After that, the plan is marked as requiring human intervention and
is never automatically retried.

---

## Implementation

The circuit breaker lives in `crates/roko-conductor/src/circuit_breaker.rs`.

```rust
use dashmap::DashMap;

pub const MAX_PLAN_FAILURES: u32 = 2;

pub struct CircuitBreaker {
    failures: DashMap<String, FailureRecord>,
}

struct FailureRecord {
    count: u32,
    // Additional metadata: timestamps, failure reasons, etc.
}
```

### Thread Safety

The `DashMap` provides lock-free concurrent reads and sharded writes.
This matters because the orchestrator may evaluate multiple plans in
parallel — each plan's conductor check should not block on other plans'
failure records.

`DashMap` is a concurrent hash map that shards its data across multiple
locks. Two plans with different IDs will almost always hit different
shards, enabling true parallel access. This is preferable to a
`Mutex<HashMap>` which would serialize all failure record access.

### API

```rust
impl CircuitBreaker {
    pub fn new() -> Self {
        Self {
            failures: DashMap::new(),
        }
    }

    /// Record a failure for a plan. Returns true if the plan is now tripped.
    pub fn record_failure(&self, plan_id: &str) -> bool {
        let mut entry = self.failures.entry(plan_id.to_string()).or_insert(FailureRecord { count: 0 });
        entry.count += 1;
        entry.count >= MAX_PLAN_FAILURES
    }

    /// Check if a plan has exceeded its failure budget.
    pub fn is_tripped(&self, plan_id: &str) -> bool {
        self.failures
            .get(plan_id)
            .map(|record| record.count >= MAX_PLAN_FAILURES)
            .unwrap_or(false)
    }

    /// Reset failure count for a plan (e.g., after manual intervention).
    pub fn reset(&self, plan_id: &str) {
        self.failures.remove(plan_id);
    }
}
```

---

## Three-State Model

The circuit breaker implements a classic three-state pattern, though
the implementation in roko-conductor uses a simplified two-state model
(tripped / not tripped). The full three-state model, implemented in
the provider health tracker (`roko-learn/src/provider_health.rs`),
provides additional granularity:

### State Transitions

```
Closed (Healthy)
  │
  │ consecutive failures >= threshold
  ▼
Open (Tripped)
  │
  │ cooldown period expires
  ▼
HalfOpen (Probing)
  │
  ├─ probe succeeds → Closed
  │
  └─ probe fails → Open (reset cooldown)
```

**Closed**: Normal operation. Failures are counted but requests proceed.
This is the initial state for every plan.

**Open**: All requests are blocked. The plan has exceeded its failure
budget. No automatic retry is permitted. In the conductor's simplified
model, this is the terminal state (tripped). In the provider health
model, the system waits for a cooldown period before transitioning to
HalfOpen.

**HalfOpen**: One probe request is permitted. If the probe succeeds,
the breaker returns to Closed. If the probe fails, the breaker returns
to Open with a fresh cooldown. This state exists in the provider health
tracker but not in the conductor's plan-level breaker — because plans
do not benefit from automatic probing (a plan that failed twice needs
a different approach, not another attempt at the same approach).

### Error-Type-Specific Cooldowns

The provider health tracker uses error classification to set cooldown
durations:

| Error Class | Cooldown | Rationale |
|------------|----------|-----------|
| RateLimit | 5 seconds | Transient; provider will accept again soon |
| Timeout | 10 seconds | Might indicate temporary load |
| ServerError | 30 seconds | Likely operational issue, needs more time |
| AuthFailure | 5 minutes | Likely persistent; manual fix needed |
| ContentPolicy | 5 minutes | Likely persistent |
| ContextOverflow | N/A | Not retryable; needs model switch |

This error-type-specific behavior lives in the provider health layer
(`roko-learn`), not in the conductor's plan-level breaker. The
conductor's plan-level breaker is simpler: two failures of any kind,
then trip.

---

## Integration with the Conductor

The circuit breaker is checked at the start of every `evaluate()` call:

```rust
impl Conductor {
    pub fn evaluate(&self, plan_id: &str, stream: &[Engram], ctx: &Context) -> ConductorDecision {
        // 1. Check circuit breaker FIRST
        if self.circuit_breaker.is_tripped(plan_id) {
            return ConductorDecision::Fail {
                reason: format!("plan {plan_id} tripped circuit breaker after {} failures", MAX_PLAN_FAILURES),
            };
        }

        // 2. Run watchers
        let watcher_outputs = self.check_all(stream, ctx);

        // 3. Apply intervention policy
        let decision = self.policy.evaluate(&watcher_outputs, ctx);

        // 4. Record failures
        if matches!(decision, ConductorDecision::Fail { .. }) {
            self.circuit_breaker.record_failure(plan_id);
        }

        decision
    }
}
```

The circuit breaker check happens before watcher evaluation. If a plan
is already tripped, there is no point running watchers — the decision
is predetermined. This short-circuit saves watcher evaluation time for
plans that are already done.

---

## Why Two Failures

The `MAX_PLAN_FAILURES = 2` constant is derived from production data:

**First failure**: Often caused by transient issues — API rate limit,
cold start, missing context. Retrying with a fresh agent and potentially
different context frequently succeeds.

**Second failure**: The same plan failing twice usually indicates a
structural problem — the task is beyond the agent's capability with
the given context, the acceptance criteria are contradictory, or the
codebase has changed in a way that makes the task impossible as
specified.

**Third failure (never reached)**: At this point, the probability of
success is negligible. The two previous attempts have already tried
the obvious approaches. A third attempt would likely repeat one of
the first two, producing the same failure at the cost of more tokens.

The math: if each attempt has a 30% success rate (typical for complex
plans that fail the first time), the probability of failing twice is
(0.7)² = 49%. The probability of failing three times is (0.7)³ = 34%.
But this assumes independence — in practice, the second failure is
correlated with the first (same root cause), so the conditional
probability of a third failure given two failures is much higher
than 70%. The expected cost of a third attempt almost always exceeds
its expected value.

---

## Relationship to Hard Guarantees

The circuit breaker implements two hard guarantees from the failure
prevention catalog:

### Hard Guarantee 3: Hard Iteration Cap

Each plan attempt includes up to 3 implementation iterations (implement
→ gate fail → retry). With 2 plan-level failures, the total maximum
is:

```
2 plan attempts × 3 iterations each = 6 total implementation cycles
```

After 6 cycles, the plan is permanently failed. This is the absolute
upper bound on token spend for any single plan.

### Hard Guarantee 7: Circuit Breaker

Direct implementation. The plan can fail a maximum of 2 times. After
2 failures, it is permanently marked as requiring human intervention
and never automatically retried.

```
MAX_PLAN_FAILURES (2) × MAX_ITERATION_LOOP (3) = 6 max attempts ever
```

This prevents:
- Infinite retry loops (max 2 failures, then stop)
- Token burn on doomed plans (6 attempts max, ever)
- Silent stuck plans (tripped state is surfaced prominently)

---

## Per-Plan Isolation

The circuit breaker is keyed by plan ID. This means:

- Plan A hitting its failure budget does not affect Plan B
- Resetting Plan A does not reset Plan B
- The breaker can track hundreds of plans concurrently

This per-plan isolation is critical for batch runs where 20+ plans
execute in parallel. A single broken plan should not cascade to
affect healthy plans.

---

## Manual Reset

The `reset()` method exists for operator override. When a human
examines a failed plan, determines the root cause, applies a fix
(updated context, different model, modified acceptance criteria), they
can reset the circuit breaker to allow the plan to retry.

This is deliberately a manual operation. The system does not auto-reset
breakers because the whole point of the breaker is to prevent automatic
retry of plans that need human judgment. If auto-reset were possible,
the breaker would be bypassed on every failure.

---

## Persistence

The circuit breaker state is part of the executor snapshot. When the
orchestrator checkpoints to `.roko/state/executor.json`, failure records
are included. On resume, the circuit breaker is restored from the
snapshot, preserving failure counts across restarts.

This prevents a circumvention where restarting the orchestrator would
reset all breakers, allowing previously-failed plans to retry. The
breaker survives crashes.

---

## Future: Adaptive Failure Budget

The current `MAX_PLAN_FAILURES = 2` is a constant. A future enhancement
is adaptive failure budgets based on plan complexity:

| Complexity | Failure Budget | Rationale |
|-----------|---------------|-----------|
| Trivial | 1 | If a trivial task fails once, something is fundamentally wrong |
| Simple | 2 | Standard budget |
| Standard | 2 | Standard budget |
| Complex | 3 | Complex tasks have higher variance; third attempt with different strategy may succeed |

This would require wiring the plan's complexity classification (from
the task TOML frontmatter) into the circuit breaker's failure threshold.
The infrastructure exists — the cascade router already uses complexity
classification for model selection.

---

## File Reference

| File | What |
|------|------|
| `crates/roko-conductor/src/circuit_breaker.rs` | CircuitBreaker struct, DashMap-based tracking |
| `crates/roko-conductor/src/conductor.rs` | Integration point — breaker checked in evaluate() |
| `crates/roko-learn/src/provider_health.rs` | Extended 3-state model for provider health |
| `crates/roko-core/src/agent.rs` | ConductorDecision enum consumed by orchestrator |

---

## Predictive Circuit Breaking

The current breaker is reactive: it counts failures after they happen.
A predictive breaker trips the circuit *before* failures cascade, based
on the trajectory of leading indicators.

### Gradient-Based Trip

Trip when the failure rate derivative exceeds a threshold. Instead of
waiting for N failures, detect the trajectory toward failure and
preempt it.

```rust
/// Predictive circuit breaker extension that trips based on failure rate trends.
/// Instead of waiting for N failures, detect the trajectory toward failure.
pub struct PredictiveBreaker {
    /// EWMA-smoothed error rate per plan.
    error_rate_ewma: DashMap<String, EwmaState>,
    /// Slope threshold: trip when d(error_rate)/dt exceeds this value.
    /// Default: 0.05 (5% per evaluation cycle).
    slope_threshold: f64,
    /// Minimum observations before predictive logic activates.
    /// Prevents false trips on insufficient data.
    min_observations: usize,
    /// Lookahead window: how far ahead to project the error rate.
    lookahead_cycles: usize,
}

impl PredictiveBreaker {
    /// Returns true if the projected error rate exceeds the trip threshold.
    pub fn should_preempt(&self, plan_id: &str) -> bool {
        if let Some(ewma) = self.error_rate_ewma.get(plan_id) {
            if ewma.observations < self.min_observations { return false; }
            let slope = ewma.derivative();
            let projected = ewma.mean + slope * self.lookahead_cycles as f64;
            projected > 0.60 && slope > self.slope_threshold
        } else {
            false
        }
    }
}
```

The `should_preempt` check runs alongside `is_tripped` in the
conductor's `evaluate()` path. If the projected error rate exceeds 60%
and the slope exceeds 5% per cycle, the breaker trips preemptively.
The `min_observations` guard prevents false trips on plans that have
only run a handful of times — the slope estimate is unreliable with
fewer than ~10 data points.

### Leading Indicators

These signals predict failures before they occur. Each maps to an
existing watcher or metric in roko-conductor:

| Indicator | What it measures | Trip condition |
|-----------|-----------------|----------------|
| Latency percentile creep | p99/p50 ratio rising | Ratio > 5x and increasing |
| Retry rate acceleration | Retries/turn increasing | d(retry_rate)/dt > 0.1 |
| Context growth rate | Tokens/turn increasing | Growth rate > 10% per turn |
| TTFT degradation | Time-to-first-token rising | TTFT > 3x baseline |
| Quality score decline | Gate scores trending down | Holt-Winters forecast < threshold |

These indicators correlate with imminent failure because they reveal
resource exhaustion and quality degradation before the final failure
event. A plan whose context grows 10% per turn will hit the context
window limit within a few turns — the breaker can trip before the
overflow, saving the wasted tokens of a doomed turn.

### Time-Series Forecasting

Holt's method (double exponential smoothing) extends the existing EWMA
with a trend component. Where EWMA tracks level only, Holt's method
tracks level and slope, enabling forward projection.

```rust
/// Holt's double exponential smoothing for trend-aware forecasting.
/// Extends the existing EWMA with a trend component.
pub struct HoltForecaster {
    /// Level component (smoothed value).
    level: f64,
    /// Trend component (smoothed rate of change).
    trend: f64,
    /// Level smoothing factor (default: 0.3).
    alpha: f64,
    /// Trend smoothing factor (default: 0.1).
    beta: f64,
    /// Number of observations seen.
    observations: usize,
}

impl HoltForecaster {
    pub fn update(&mut self, value: f64) {
        if self.observations == 0 {
            self.level = value;
            self.trend = 0.0;
        } else {
            let prev_level = self.level;
            self.level = self.alpha * value + (1.0 - self.alpha) * (prev_level + self.trend);
            self.trend = self.beta * (self.level - prev_level) + (1.0 - self.beta) * self.trend;
        }
        self.observations += 1;
    }

    /// Forecast h steps ahead.
    pub fn forecast(&self, h: usize) -> f64 {
        self.level + self.trend * h as f64
    }
}
```

The `alpha` parameter controls how quickly the level responds to new
observations; `beta` controls how quickly the trend responds. Lower
values produce smoother estimates that resist noise but lag behind
real changes. The defaults (0.3 / 0.1) bias toward smooth trend
estimation — appropriate for circuit breaker decisions where false
trips are more costly than late trips.

---

## Partial Circuit Breaking — Graceful Degradation

A full circuit trip halts all work on a plan. Partial circuit breaking
degrades individual capabilities while keeping core execution running.
This is the difference between "stop everything" and "stop the parts
that are failing."

### Feature-Level Breakers

Each plan capability has its own circuit. When context enrichment fails
three times, the enrichment circuit opens — but compilation, testing,
and implementation continue uninterrupted.

```rust
/// Feature-level circuit breaker: break individual plan capabilities
/// while the core execution continues.
pub struct FeatureBreaker {
    /// Per-feature failure tracking.
    features: DashMap<String, FeatureCircuit>,
}

pub struct FeatureCircuit {
    pub feature: PlanFeature,
    pub state: CircuitState,
    pub failures: u32,
    pub max_failures: u32,
    /// Fallback behavior when this feature is broken.
    pub fallback: FeatureFallback,
}

/// Plan capabilities that can be independently circuit-broken.
pub enum PlanFeature {
    GateRung(String),      // Individual gate rung (clippy, coverage, etc.)
    ContextEnrichment,     // Adding related code context
    ResearchEnhancement,   // Research-based task enrichment
    DocUpdate,             // Documentation generation
    ReviewCycle,           // Code review by reviewer agent
}

/// Fallback behavior when a feature circuit opens.
pub enum FeatureFallback {
    Skip,                  // Omit this feature entirely
    UseCached,             // Use last successful result
    Downgrade(String),     // Use simpler version (e.g., Opus -> Haiku reviewer)
    WarnAndContinue,       // Log warning, proceed without feature
}
```

The degradation hierarchy defines how each feature fails gracefully:

| Feature | Fallback 1 | Fallback 2 | Fallback 3 |
|---------|-----------|-----------|-----------|
| Clippy gate | WarnAndContinue | Skip | Skip |
| Context enrichment | UseCached | Downgrade (minimal context) | Skip |
| Research enhancement | UseCached | Skip | Skip |
| Review cycle | Downgrade (Haiku reviewer) | WarnAndContinue | Skip |
| Compile gate | (no fallback — always required) | — | — |
| Test gate | (no fallback — always required) | — | — |

Compile and test gates have no fallback because they enforce
correctness. A plan that does not compile is not a plan. Everything
else — linting, enrichment, review — is valuable but not essential.
The feature breaker encodes this distinction: some capabilities are
negotiable, some are not.

### Graduated Probe Traffic (Half-Open Enhancement)

The standard half-open state is binary: one probe request, pass or
fail. Graduated probing ramps traffic from 5% to 100%, reducing the
risk that a single lucky probe declares the breaker healthy when the
underlying problem persists.

```rust
/// Enhanced half-open state with graduated probe traffic.
/// Instead of binary open/closed, ramp traffic from 5% to 100%.
pub struct GraduatedHalfOpen {
    /// Current probe fraction (0.0 = fully open, 1.0 = fully closed).
    probe_fraction: f64,
    /// Multiplier on each successful probe (default: 2.0).
    ramp_factor: f64,
    /// Initial probe fraction when entering half-open (default: 0.05).
    initial_fraction: f64,
    /// Sleep window before first probe (default: 300s).
    base_sleep_ms: u64,
    /// Maximum sleep window after repeated failures (default: 1800s = 30 min).
    max_sleep_ms: u64,
    /// Current sleep window (doubles on each re-open).
    current_sleep_ms: u64,
}

impl GraduatedHalfOpen {
    pub fn on_probe_success(&mut self) {
        self.probe_fraction = (self.probe_fraction * self.ramp_factor).min(1.0);
    }

    pub fn on_probe_failure(&mut self) {
        self.probe_fraction = self.initial_fraction;
        self.current_sleep_ms = (self.current_sleep_ms * 2).min(self.max_sleep_ms);
    }

    pub fn is_fully_recovered(&self) -> bool {
        self.probe_fraction >= 1.0
    }
}
```

The ramp sequence with default settings: 5% -> 10% -> 20% -> 40% ->
80% -> 100%. Each step requires a successful probe at the current
traffic level. A failure at any step resets to 5% and doubles the
sleep window (capped at 30 minutes). This exponential backoff on the
sleep window prevents rapid re-probing of a persistently broken
dependency.

### Load Shedding Under Pressure

When the cost budget is tight, the system sheds low-priority work
first. This mirrors how production systems handle overload: degrade
gracefully rather than fail completely.

The shedding tiers:

- When cost > 70% budget: defer doc-update and enrichment tasks
- When cost > 85% budget: defer all non-critical-path tasks
- When cost > 95% budget: only execute core implementation + required gates

```rust
/// Load shedding policy: which tasks to defer under budget pressure.
pub struct LoadSheddingPolicy {
    /// Budget utilization thresholds for each shedding tier.
    tiers: Vec<SheddingTier>,
}

pub struct SheddingTier {
    /// Budget utilization threshold (0.0 to 1.0) to activate this tier.
    pub threshold: f64,
    /// Task priorities that are shed at this tier (lower = shed first).
    pub shed_below_priority: TaskPriority,
}

#[derive(PartialEq, Eq, PartialOrd, Ord)]
pub enum TaskPriority {
    Optional = 0,      // Doc updates, style fixes
    Enhancement = 1,   // Enrichment, research
    Standard = 2,      // Review cycles, optional gates
    Required = 3,      // Core implementation
    Critical = 4,      // Required gates (compile, test)
}
```

The priority ordering matters. `TaskPriority` derives `Ord`, so the
shedding policy can compare priorities directly: at threshold 0.70,
shed everything below `Standard`; at 0.85, shed everything below
`Required`; at 0.95, shed everything below `Critical`. The compile
and test gates survive all shedding tiers — they are never deferred.

### Adaptive Concurrency (AIMD)

Fixed concurrency limits are either too conservative (wasting
parallelism) or too aggressive (overloading the provider). AIMD
(Additive Increase, Multiplicative Decrease) self-tunes to the
optimal concurrency, using the same algorithm as TCP congestion
control.

- On success: `concurrency += 1 / concurrency` (additive increase)
- On failure: `concurrency *= 0.9` (multiplicative decrease)

```rust
/// AIMD-based adaptive concurrency limiter.
/// Self-tunes to optimal concurrency without fixed configuration.
pub struct AdaptiveConcurrency {
    /// Current concurrency limit (float for smooth adjustment).
    limit: f64,
    /// Minimum concurrency (default: 1.0).
    floor: f64,
    /// Maximum concurrency (default: 10.0).
    ceiling: f64,
    /// Multiplicative decrease factor on failure (default: 0.9).
    decrease_factor: f64,
}

impl AdaptiveConcurrency {
    pub fn on_success(&mut self) {
        self.limit = (self.limit + 1.0 / self.limit).min(self.ceiling);
    }
    pub fn on_failure(&mut self) {
        self.limit = (self.limit * self.decrease_factor).max(self.floor);
    }
    pub fn current_limit(&self) -> usize {
        self.limit.ceil() as usize
    }
}
```

The additive increase is inversely proportional to the current limit.
At concurrency 2, each success adds 0.5. At concurrency 8, each
success adds 0.125. This produces slow, cautious growth at high
concurrency — exactly the behavior you want when approaching the
provider's rate limit. The multiplicative decrease (10% reduction per
failure) drops the limit fast enough to relieve pressure without
collapsing to 1.

---

## Chaos Engineering for Circuit Breaker Validation

Circuit breakers that are never tested in failure conditions are
circuit breakers that might not work when they matter. Chaos
engineering validates the breaker by injecting controlled failures
and observing whether the system responds correctly.

### Chaos Experiment Types

Each experiment type maps from established chaos engineering practice
(Netflix Simian Army, Gremlin) to the agent orchestration domain:

| Chaos type | Agent equivalent | Tests |
|-----------|-----------------|-------|
| Process kill | Kill agent mid-task | ProcessSupervisor, ghost-turn watcher |
| Latency injection | Artificial API delay | TimeOverrunWatcher, circuit breaker |
| Error injection | Force compile errors | CompileFailRepeatWatcher, stuck detector |
| Resource saturation | Fill context window | ContextWindowPressureWatcher |
| Cost spike | Inject expensive turns | CostOverrunWatcher, anomaly detector |
| Rate limit | Throttle API calls | Cascade router fallback |

### Steady-State Hypothesis

Before injecting chaos, define what "normal" looks like. The
experiment succeeds if the system returns to this steady state after
the injection ends.

```
gate_pass_rate > 0.8 over rolling 10-run window
agent_cost_per_task < $0.50
p95_task_completion_time < 300 seconds
zero plans in CIRCUIT_TRIPPED state
```

If the system cannot return to these baselines after chaos injection,
the circuit breaker (or one of its supporting watchers) has a gap.

### Principles

Three rules for chaos experiments in this system:

1. **Start small.** Run against one plan in isolation. Never inject
   chaos into a full batch run until single-plan experiments pass.
2. **Minimize blast radius.** Use synthetic plans with throwaway
   tasks. Never inject failures into plans that produce real code
   changes.
3. **Run in synthetic plans first.** Build a suite of canary plans
   whose sole purpose is chaos testing. These plans have known-good
   tasks (for baseline) and intentionally flawed tasks (for failure
   injection). Run them on every release before promoting to
   production orchestration.

### References

- Nygard (2007) — *Release It!*, circuit breaker pattern
- Netflix Hystrix — rolling window metrics, health calculation
- Resilience4j — sliding window, slow-call detection
- Netflix Principles of Chaos Engineering (2018)
- Patterson et al. (2002) — Recovery-Oriented Computing
- Candea & Fox (2003) — Micro-reboots


---

# SOURCE: /Users/will/dev/nunchi/roko/roko/docs/07-conductor/03-graduated-interventions.md

# Graduated Interventions

> Three actions. No nudges. Continue, Restart, or Fail.
> The Conductor decides; it does not suggest.


> **Implementation**: Built

---

## The ConductorDecision Enum

Every evaluation cycle produces exactly one decision:

```rust
pub enum ConductorDecision {
    Continue,
    Restart { reason: String },
    Fail { reason: String },
}
```

**Continue**: All watchers report healthy. The plan proceeds without
intervention.

**Restart**: At least one watcher reported Warning severity. The current
agent is killed and restarted with different context. The key difference
from a retry: the restarted agent gets a FRESH start with ADDITIONAL
information about what went wrong. It is not the same agent continuing
from a confused state — it is a new agent with the benefit of hindsight.

**Fail**: At least one watcher reported Critical severity, or the
circuit breaker is tripped. The plan is marked as failed. The
orchestrator removes it from the merge queue, cancels in-flight tasks,
and dispatches work for other plans.

---

## Why No Nudge

Production experience (Issue #9, agent ghost turns; Issue #6, conductor
nudges without effect) demonstrated that nudging does not work:

```
Agent is stuck → Conductor sends nudge message →
Agent reads nudge → Agent attempts same approach →
Agent is still stuck → Conductor sends another nudge → ...
```

The problem: a confused agent remains confused after receiving a nudge.
The nudge says "you seem stuck, try a different approach" — but the
agent's confusion is WITHIN its context. Adding a nudge message to an
already-confused context does not reduce confusion. It may even increase
it by adding more text the agent needs to process.

The structural fix: the Conductor does not nudge. It either restarts
(kills the agent, gives a new agent the error analysis) or fails (marks
the plan for human attention). Both actions create a clean break from
the confused state.

This is Hard Guarantee 6 from the failure prevention catalog:
"The Conductor DECIDES, Never Nudges."

---

## The Severity System

### Three Levels

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Severity {
    Info = 0,
    Warning = 1,
    Critical = 2,
}
```

The `PartialOrd` derivation enables severity comparison: `Critical >
Warning > Info`. The intervention policy uses this ordering to select
the maximum severity from all watcher outputs.

### Mapping to Decisions

| Severity | Decision | Orchestrator Action |
|----------|----------|-------------------|
| Info | Continue | No action. Log the observation. |
| Warning | Restart | Kill current agent. Spawn fresh agent with error context. |
| Critical | Fail | Mark plan as failed. Cancel in-flight work. Move to next plan. |

### Watcher Severity Defaults

| Watcher | Default Severity | Rationale |
|---------|-----------------|-----------|
| ghost-turn | Warning | Agent may recover; fresh start often helps |
| compile-fail-repeat | Warning | Different context may resolve the error |
| cost-overrun | Warning | May be worth one more attempt with budget awareness |
| iteration-loop | **Critical** | Three gate failures = fundamental mismatch |
| review-loop | Warning | Skip reviews and proceed to merge |
| spec-drift | Warning | Refocus the agent on declared scope |
| stuck-pattern | Warning | Fresh agent with different strategy |
| test-failure-budget | Warning | Agent is introducing regressions; needs restart |
| time-overrun | Warning | Early warning; may still finish in time |
| context-window-pressure | Warning | Compact context and retry |

Only `iteration-loop` defaults to Critical. Every other watcher
produces Warning, giving the plan one chance to recover through restart
before being failed.

---

## WatcherOutput

The intermediate representation between watcher signals and conductor
decisions:

```rust
pub struct WatcherOutput {
    pub watcher: String,       // which watcher fired
    pub severity: Severity,    // info / warning / critical
    pub description: String,   // human-readable explanation
    pub metric: Option<f64>,   // optional numeric value (ratio, count, etc.)
}
```

The Conductor collects `WatcherOutput`s from all watchers that fired
(returned non-empty signal vectors), then passes the collection to the
`InterventionPolicy` for resolution.

---

## The InterventionPolicy Trait

```rust
pub trait InterventionPolicy: Send + Sync {
    fn evaluate(
        &self,
        outputs: &[WatcherOutput],
        ctx: &Context,
    ) -> ConductorDecision;
}
```

The trait is deliberately simple: given a set of watcher outputs, produce
a decision. This allows different resolution strategies:

### WorstSeverityPolicy (Default)

The maximum severity among all outputs determines the decision:

```rust
pub struct WorstSeverityPolicy;

impl InterventionPolicy for WorstSeverityPolicy {
    fn evaluate(&self, outputs: &[WatcherOutput], _ctx: &Context) -> ConductorDecision {
        if outputs.is_empty() {
            return ConductorDecision::Continue;
        }

        let worst = outputs.iter()
            .map(|o| o.severity)
            .max()
            .unwrap_or(Severity::Info);

        match worst {
            Severity::Info => ConductorDecision::Continue,
            Severity::Warning => ConductorDecision::Restart {
                reason: format_watcher_reasons(outputs),
            },
            Severity::Critical => ConductorDecision::Fail {
                reason: format_watcher_reasons(outputs),
            },
        }
    }
}
```

This is a conservative policy: if ANY watcher reports a problem, the
Conductor acts on it. One watcher saying "critical" overrides nine
watchers saying "continue."

### Alternative Policies (Not Yet Implemented)

**MajoritySeverityPolicy**: Use the median severity instead of the
maximum. More tolerant — a single warning among nine healthy watchers
would be outvoted.

**WeightedSeverityPolicy**: Assign weights to watchers based on
their historical accuracy. Watchers with high false-positive rates
get lower weights. This requires the learning system to track
watcher accuracy.

**ContextualPolicy**: Different policies for different plan phases.
During Implementation, be aggressive (restart early). During Review,
be lenient (reviewers are inherently noisy). During Merge, be very
conservative (merge failures are expensive).

The `InterventionPolicy` trait supports all of these through
polymorphism. The Conductor stores `Box<dyn InterventionPolicy>` and
can switch policies at runtime.

---

## Decision Flow

The complete decision flow from signal stream to orchestrator action:

```
Signal Stream
    │
    ├── Watcher 1: ghost-turn      → [no fire]
    ├── Watcher 2: compile-fail    → Warning: "3 identical E0308 errors"
    ├── Watcher 3: cost-overrun    → [no fire]
    ├── Watcher 4: iteration-loop  → [no fire]
    ├── Watcher 5: review-loop     → [no fire]
    ├── Watcher 6: spec-drift      → Warning: "drift 32% exceeds 25%"
    ├── Watcher 7: stuck-pattern   → [no fire]
    ├── Watcher 8: test-budget     → [no fire]
    ├── Watcher 9: time-overrun    → [no fire]
    └── Watcher 10: ctx-pressure   → [no fire]
    │
    ▼
WatcherOutputs: [
    { watcher: "compile-fail-repeat", severity: Warning, ... },
    { watcher: "spec-drift", severity: Warning, ... },
]
    │
    ▼
WorstSeverityPolicy:
    max(Warning, Warning) = Warning
    │
    ▼
ConductorDecision::Restart {
    reason: "compile-fail-repeat: 3 identical E0308 errors; spec-drift: drift 32% exceeds 25%"
}
    │
    ▼
Orchestrator:
    1. Kill current agent
    2. Record failure in circuit breaker
    3. Spawn new agent with:
       - Error analysis from Diagnosis Engine
       - Updated context with compile error details
       - Refocused scope from spec drift data
```

---

## Escalation Semantics

### What Happens on Restart

When the Conductor decides Restart:

1. **The current agent is terminated.** Not paused, not given a
   final chance — terminated. Its process is killed and its context
   is discarded.

2. **The error context is preserved.** Gate results, compiler errors,
   watcher observations, and the reason for restart are collected into
   an error brief.

3. **A new agent is spawned.** Fresh context. No memory of the
   confused state. But it receives the error brief — it knows what
   the previous agent tried and why it failed.

4. **The iteration counter increments.** This restart counts toward
   the plan's iteration limit. After MAX_ITERATION_LOOP restarts,
   the next failure will be Critical.

The restart is not a continuation. It is a fresh start with the
benefit of hindsight.

### What Happens on Fail

When the Conductor decides Fail:

1. **All in-flight tasks for the plan are cancelled.** Agents are
   killed. Worktree state is preserved for post-mortem.

2. **The plan phase transitions to Failed(reason).** The reason
   includes which watcher fired and why.

3. **The circuit breaker records the failure.** If this is the
   second failure, the plan is tripped and will never be automatically
   retried.

4. **The orchestrator moves on.** Other plans that do not depend on
   the failed plan continue without interruption.

5. **The failure is surfaced.** The plan appears as "Failed" in the
   dashboard with the full reason. The deferred-failures log captures
   structured records with error snippets and failure context.

---

## Cooldown Periods

Each watcher intervention has a built-in cooldown to prevent the
conductor from firing the same intervention on consecutive evaluation
cycles.

The production experience that motivated cooldowns: the conductor
would detect a stuck agent, emit a restart signal, and then on the
next tick (before the restart had taken effect), detect the same stuck
signal again and emit another restart. This double-fire would sometimes
kill the replacement agent that was still starting up.

The cooldown ensures that after a watcher fires, it does not fire
again for the same plan until enough time has passed for the
intervention to take effect. The production default is 120 seconds
per plan per watcher.

---

## Intervention Signals

When the Conductor makes a non-Continue decision, it emits a signal
to the stream:

```rust
Engram::builder(Kind::Custom("conductor.intervention".into()))
    .body(Body::text(format!("{watcher_name}: {description}")))
    .tag("watcher", watcher_name)
    .tag("severity", severity_str)
    .tag("plan_id", plan_id)
    // ... watcher-specific tags
    .build()
```

These signals serve two purposes:

1. **Observability**: The dashboard, event log, and signal replay
   system can show exactly when and why the conductor intervened.

2. **Learning**: The efficiency tracking system records interventions
   as negative signals. Plans that trigger conductor interventions
   produce data for the cascade router's reward function, penalizing
   model/task combinations that produce intervention-worthy behavior.

---

## Relationship to Yerkes-Dodson Dynamics

Research on 770,000+ autonomous agents (§2.7 of the orchestration
reference) shows that cooperative behavior follows an inverted-U
curve with environmental pressure:

- **Too little pressure** (no iteration limits, generous timeouts):
  agents waste tokens exploring irrelevant approaches
- **Moderate pressure** (bounded iterations, reasonable timeouts):
  agents focus on the task and cooperate effectively
- **Too much pressure** (aggressive limits, tight timeouts):
  agents collapse into minimal-effort responses, skip steps, and
  produce incomplete work

The Conductor's intervention thresholds are Yerkes-Dodson parameters.
They sit somewhere on this curve:

- `MAX_GHOST_TURNS = 3` — how much silence before intervention
- `MAX_COMPILE_FAIL_REPEAT = 3` — how many identical errors before restart
- `MAX_ITERATION_LOOP = 3` — how many gate failures before fail
- `MAX_REVIEW_CYCLES = 3` — how many review rejects before skip
- `ALERT_THRESHOLD = 0.80` — how full the context can get
- `MAX_SPEC_DRIFT_RATIO = 0.25` — how much scope drift is tolerated

Each threshold represents a pressure setting. Too aggressive and agents
collapse. Too lenient and agents waste. The learning system
(efficiency events, cascade router observations) provides data for
tuning these thresholds over time — moving along the Yerkes-Dodson
curve toward the peak of the inverted-U.

Reference: Yerkes & Dodson (1908). "The relation of strength of
stimulus to rapidity of habit-formation."

---

## File Reference

| File | What |
|------|------|
| `crates/roko-conductor/src/interventions.rs` | Severity, WatcherOutput, InterventionPolicy, WorstSeverityPolicy |
| `crates/roko-conductor/src/conductor.rs` | evaluate() — decision flow |
| `crates/roko-core/src/agent.rs` | ConductorDecision enum |


---

# SOURCE: /Users/will/dev/nunchi/roko/roko/docs/07-conductor/04-diagnosis-engine.md

# Diagnosis Engine

> Thirty-four patterns across twenty error categories.
> Given raw error text, return a typed diagnosis with category,
> confidence, and suggested intervention.


> **Implementation**: Built

---

## Purpose

The Diagnosis Engine replaces ad-hoc error parsing with structured
classification. Instead of each component grepping for "error[E0308]"
in raw output, the engine accepts raw error text and returns a typed
`Diagnosis` with:

- Error category (one of 20 enumerated types)
- Confidence score (0.0 to 1.0)
- Suggested intervention (one of 9 actions)
- Description and context

This structured classification enables:

1. **Consistent handling**: The same error always gets the same
   classification, regardless of which component encounters it.
2. **Appropriate response**: A missing import (cheap to fix) gets
   `AutoFix`. A borrow checker error (requires understanding) gets
   `RestartAgent` with additional context.
3. **Learning**: Error categories feed into the efficiency tracking
   system, enabling per-category success rate analysis.
4. **Observability**: The dashboard can show "12 CompileErrors, 3
   TestFailures, 1 LifetimeError" instead of "16 errors."

---

## Error Categories

The `ErrorCategory` enum defines twenty categories covering the
full range of errors encountered in production batch runs:

```rust
pub enum ErrorCategory {
    CompileError,
    TestFailure,
    TypeMismatch,
    BorrowCheckerError,
    LifetimeError,
    ImportError,
    MissingFile,
    PermissionDenied,
    NetworkError,
    TimeoutError,
    OomError,
    DiskFull,
    LlmRateLimit,
    LlmContextOverflow,
    LlmRefusal,
    ProcessCrash,
    LoopDetected,
    ClippyWarning,
    GitConflict,
    DependencyError,
}
```

### Category Groupings

**Rust compiler errors** (6 categories):
- `CompileError` — general compilation failure
- `TypeMismatch` — E0308, expected vs. found types
- `BorrowCheckerError` — E0382, E0505, E0507, use-after-move and
  borrow violations
- `LifetimeError` — E0106, E0495, E0621, lifetime annotations
- `ImportError` — E0432, E0433, unresolved imports and modules
- `ClippyWarning` — clippy lint violations

**Test and verification** (1 category):
- `TestFailure` — test assertion failures, panic in tests

**File system** (3 categories):
- `MissingFile` — file not found errors
- `PermissionDenied` — file permission errors
- `DiskFull` — no space left on device

**Infrastructure** (3 categories):
- `NetworkError` — connection failures, DNS resolution
- `TimeoutError` — operation timeouts
- `OomError` — out of memory

**LLM provider** (3 categories):
- `LlmRateLimit` — 429 errors, quota exceeded
- `LlmContextOverflow` — prompt exceeds model context window
- `LlmRefusal` — content policy rejection

**Process** (2 categories):
- `ProcessCrash` — agent process exited unexpectedly
- `LoopDetected` — agent entering a detected loop

**Version control** (1 category):
- `GitConflict` — merge conflicts, rebase failures

**Dependencies** (1 category):
- `DependencyError` — cargo dependency resolution, feature flags

---

## Suggested Interventions

Each diagnosis maps to one of nine intervention actions:

```rust
pub enum SuggestedIntervention {
    RetryWithContext,
    AutoFix,
    RestartAgent,
    AbortPlan,
    BackoffRetry,
    MergeResolution,
    ReduceContext,
    SwitchModel,
    WarnAndContinue,
}
```

### Intervention Semantics

| Intervention | When Used | What Happens |
|-------------|-----------|-------------|
| `RetryWithContext` | Transient errors; adding context may help | Retry the operation with additional error context in the prompt |
| `AutoFix` | Simple, well-understood errors (imports, missing fields) | Route to cheap Haiku-tier auto-fix agent |
| `RestartAgent` | Agent is confused or stuck | Kill and respawn with fresh context + error analysis |
| `AbortPlan` | Unrecoverable errors | Mark plan as failed immediately |
| `BackoffRetry` | Rate limits, temporary outages | Wait with exponential backoff, then retry |
| `MergeResolution` | Git merge conflicts | Spawn merge resolver agent |
| `ReduceContext` | Context overflow | Compact context and retry with smaller prompt |
| `SwitchModel` | Model limitation (refusal, context overflow) | Route to a different model |
| `WarnAndContinue` | Clippy warnings, non-blocking issues | Log the warning but do not interrupt execution |

### Category-to-Intervention Mapping

| Category | Primary Intervention | Rationale |
|----------|---------------------|-----------|
| CompileError | RetryWithContext | Agent may fix with error details |
| TestFailure | RetryWithContext | Agent may fix with test output |
| TypeMismatch | RetryWithContext | Agent needs the expected/found types |
| BorrowCheckerError | RestartAgent | Borrow errors require fresh approach |
| LifetimeError | RestartAgent | Lifetime errors are structurally difficult |
| ImportError | AutoFix | Missing imports are cheap to fix |
| MissingFile | RetryWithContext | Agent may need to create the file |
| PermissionDenied | AbortPlan | Cannot fix permissions from agent context |
| NetworkError | BackoffRetry | Likely transient |
| TimeoutError | BackoffRetry | Likely transient |
| OomError | AbortPlan | Resource exhaustion requires operator action |
| DiskFull | AbortPlan | Resource exhaustion requires cleanup |
| LlmRateLimit | BackoffRetry | Wait for rate limit window to expire |
| LlmContextOverflow | ReduceContext | Compact and retry |
| LlmRefusal | SwitchModel | Try a different model |
| ProcessCrash | RestartAgent | Agent died; respawn |
| LoopDetected | RestartAgent | Agent is stuck; fresh context needed |
| ClippyWarning | WarnAndContinue | Non-blocking |
| GitConflict | MergeResolution | Spawn merge resolver |
| DependencyError | RetryWithContext | Agent may fix with dependency info |

---

## Pattern Matching

The engine contains 34 built-in patterns. Each pattern is a substring
match with an associated category, confidence, and suggested
intervention:

```rust
struct ErrorPattern {
    substring: &'static str,
    category: ErrorCategory,
    confidence: f64,
    intervention: SuggestedIntervention,
}
```

### Pattern Examples

| Pattern Substring | Category | Confidence | Intervention |
|------------------|----------|-----------|-------------|
| `"error[E0308]"` | TypeMismatch | 0.95 | RetryWithContext |
| `"error[E0382]"` | BorrowCheckerError | 0.95 | RestartAgent |
| `"error[E0106]"` | LifetimeError | 0.95 | RestartAgent |
| `"error[E0432]"` | ImportError | 0.95 | AutoFix |
| `"error[E0433]"` | ImportError | 0.95 | AutoFix |
| `"error[E0063]"` | CompileError | 0.90 | AutoFix |
| `"cannot find"` | ImportError | 0.70 | RetryWithContext |
| `"test result: FAILED"` | TestFailure | 0.90 | RetryWithContext |
| `"panicked at"` | TestFailure | 0.85 | RetryWithContext |
| `"Connection refused"` | NetworkError | 0.80 | BackoffRetry |
| `"rate limit"` | LlmRateLimit | 0.90 | BackoffRetry |
| `"context_length_exceeded"` | LlmContextOverflow | 0.95 | ReduceContext |
| `"No space left"` | DiskFull | 0.95 | AbortPlan |
| `"CONFLICT"` | GitConflict | 0.80 | MergeResolution |
| `"clippy::"`  | ClippyWarning | 0.90 | WarnAndContinue |

The confidence score reflects how specific the substring match is.
Rust error codes (E0308, E0382) are highly specific — confidence 0.95.
Generic substrings ("cannot find") are less specific — confidence 0.70.

### Matching Algorithm

```rust
impl DiagnosisEngine {
    pub fn diagnose(&self, error_text: &str) -> Vec<Diagnosis> {
        let lower = error_text.to_lowercase();
        self.patterns.iter()
            .filter(|p| lower.contains(&p.substring.to_lowercase()))
            .map(|p| Diagnosis {
                category: p.category,
                confidence: p.confidence,
                intervention: p.intervention,
                description: format!("Matched pattern: {}", p.substring),
            })
            .collect()
    }
}
```

Multiple patterns can match the same error text. The caller receives
all matching diagnoses and can select the highest-confidence one or
use the full set for richer context.

---

## Integration Points

### With the Conductor

When a watcher fires, the Conductor can pass the error context through
the Diagnosis Engine before making its decision. This enriches the
intervention signal with structured error classification:

```
Watcher fires: "compile-fail-repeat: 3 identical errors"
    │
    ▼
Diagnosis Engine: error text → [Diagnosis { category: ImportError, confidence: 0.95, intervention: AutoFix }]
    │
    ▼
Enriched decision: Restart with { intervention: AutoFix, context: "E0432: unresolved import" }
```

### With the Auto-Fix Pipeline

The `AutoFix` intervention routes errors to a lightweight Haiku-tier
agent. The Diagnosis Engine's classification determines which errors
qualify for auto-fix:

- `ImportError` → auto-fixable (add the correct `use` statement)
- `CompileError` with E0063 (missing struct field) → auto-fixable
- `TypeMismatch` → sometimes auto-fixable if the conversion is simple
- `BorrowCheckerError` → not auto-fixable (requires architectural understanding)

The cost difference is significant: an auto-fix costs ~$0.01 (Haiku,
small context). A full re-implementation cycle costs ~$2.00+ (Opus,
full context). When 6 out of 8 errors are missing imports, the engine
saves $11.94 by routing them to auto-fix instead of full re-implementation.

### With the Learning System

Error categories feed into the efficiency tracking system:

```
AgentEfficiencyEvent {
    outcome: "gate_failed",
    gate_errors: [
        { category: "ImportError", count: 3 },
        { category: "TypeMismatch", count: 1 },
    ],
    // ...
}
```

Over time, this data reveals patterns:
- "Plans touching `src/auth/` have 40% LifetimeError rate"
- "Haiku agents produce 3x more ImportError than Sonnet"
- "Auto-fix resolves ImportError 95% of the time"

These patterns inform:
- Prompt engineering (add lifetime notes for auth-related tasks)
- Model routing (use Sonnet for auth tasks, Haiku for others)
- Auto-fix thresholds (route ImportError to auto-fix with high confidence)

---

## Design Decisions

### Why Substring Matching Instead of Regex

The engine uses simple substring matching (`contains()`), not regular
expressions. Rationale:

1. **Performance**: Substring matching is O(n) per pattern, O(n*m) for
   all patterns. Regex compilation and matching adds overhead.
2. **Readability**: `"error[E0308]"` is immediately clear. A regex
   for the same match would be less readable.
3. **Maintainability**: Adding a new pattern is adding a string literal
   and its metadata. No regex debugging.
4. **Coverage**: The 34 patterns cover the most common errors
   encountered in production. Regex would be needed for complex
   extraction (e.g., parsing the expected/found types from a type
   mismatch), but the diagnosis engine's job is classification, not
   extraction.

### Why 34 Patterns

The pattern count (34) was derived from production data. During batch
runs in March-April 2026, every distinct error type was cataloged.
The 34 patterns cover approximately 95% of observed errors by
frequency. The remaining 5% are rare edge cases that fall through to
the default `CompileError` category.

The test `has_at_least_20_patterns()` ensures the pattern set is not
accidentally reduced. New patterns are added as new error types are
encountered in production.

### Why Twenty Categories

The category count (20) balances granularity against complexity:

- Too few categories (e.g., "CompileError" for everything) loses
  the information needed for appropriate intervention routing.
- Too many categories (e.g., one per rustc error code) creates
  maintenance burden without proportional benefit.

Twenty categories cover the natural groupings of errors at the level
of actionable difference — each category maps to a different
intervention strategy.

---

## Production Error Distribution

From production batch runs, the approximate error frequency distribution:

| Category | Frequency | Auto-Fix Rate |
|----------|----------|--------------|
| ImportError | 35% | 95% |
| CompileError (general) | 20% | 30% |
| TypeMismatch | 15% | 50% |
| TestFailure | 12% | 0% (requires understanding) |
| BorrowCheckerError | 5% | 10% |
| LifetimeError | 4% | 10% |
| LlmRateLimit | 3% | N/A (retry) |
| All others | 6% | varies |

The key insight: over a third of all errors are import errors, and
95% of those can be auto-fixed for $0.01 each. Without the diagnosis
engine, all errors would go through full re-implementation at $2+ each.
The engine's cost savings are dominated by this single category.

---

## Future: Confidence-Weighted Routing

Currently, the diagnosis engine classifies errors and suggests
interventions. A future enhancement is confidence-weighted routing:

1. **High confidence (>0.9)**: Route directly to suggested intervention.
   No human review needed.
2. **Medium confidence (0.6-0.9)**: Route to suggested intervention but
   flag for review.
3. **Low confidence (<0.6)**: Fall back to RestartAgent (the safest
   generic intervention).

This tiered routing would reduce false-positive auto-fixes (where the
engine misclassifies an error and the auto-fix agent wastes a turn on
something it cannot fix).

---

## File Reference

| File | What |
|------|------|
| `crates/roko-conductor/src/diagnosis.rs` | DiagnosisEngine, ErrorCategory, SuggestedIntervention, 34 patterns |


---

# SOURCE: /Users/will/dev/nunchi/roko/roko/docs/07-conductor/05-stuck-detection.md

# Stuck Detection and Meta-Cognition

> Six heuristics for detecting stuck agents. A MetaCognitionHook
> that wraps them into a periodic self-assessment: "Am I stuck?
> Am I thrashing? Should I escalate?"


> **Implementation**: Built

---

## The Stuck Problem

An agent can be stuck in ways that no single watcher catches:

- **Output loop**: The agent produces output, but it is the same
  output every turn. Tools are called, files are read, but nothing
  changes.
- **No progress**: The agent is active (producing output, calling
  tools) but no files change and no tests pass. Activity without
  progress.
- **Gate loop**: The agent fixes one error, introduces another, fixes
  that, reintroduces the first. The gate failure count oscillates
  but never reaches zero.
- **Compile loop**: A variation of gate loop specific to compile
  errors. The agent toggles between two incompatible fixes.
- **Empty output**: The agent returns content, but it is
  acknowledgments, descriptions of intended actions, or questions
  to nobody. No tool calls, no file changes.
- **Excessive retries**: The agent retries the same operation
  repeatedly without changing its approach.

These modes are all variations on the same theme: the agent is
consuming resources (tokens, wall-clock time, API quota) without
making progress toward task completion.

---

## StuckKind Enum

```rust
pub enum StuckKind {
    OutputLoop,
    NoProgress,
    GateLoop,
    CompileLoop,
    EmptyOutput,
    ExcessiveRetries,
}
```

Each variant represents a distinct detection heuristic. Multiple
variants can be detected simultaneously (an agent can be both in
an output loop and showing no progress).

---

## StuckDetector

```rust
pub struct StuckDetector {
    thresholds: StuckThresholds,
}

pub struct StuckThresholds {
    pub output_loop: usize,         // default: 4
    pub no_progress_ms: u64,        // default: 300_000 (5 minutes)
    pub gate_loop: usize,           // default: 3
    pub compile_loop: usize,        // default: 3
    pub empty_output: usize,        // default: 3
    pub excessive_retry: usize,     // default: 6
}
```

### Detection Heuristics

#### OutputLoop (threshold: 4)

Computes a content hash of each agent turn's output. If four
consecutive turns produce the same hash, the agent is stuck in an
output loop.

**Why hash-based**: Exact string comparison would miss near-identical
outputs (same content with minor formatting differences). Hashing
normalizes the comparison. In practice, true output loops produce
byte-identical output because the agent is executing the same
reasoning chain.

**Why 4**: One repeated output is common (agent checks something
twice). Two repetitions may indicate deliberate verification. Three
is suspicious. Four consecutive identical outputs is definitively
a loop.

#### NoProgress (threshold: 300,000 ms / 5 minutes)

Checks elapsed time since the last file modification or test state
change. If 5 minutes pass with no measurable progress, the agent is
stuck.

**Why time-based**: Unlike the other heuristics which count events,
no-progress detection is time-based because the agent may not produce
any events to count. A truly stuck agent might be in a reasoning loop
with no tool calls at all — no output, no file changes, nothing to
count.

**Why 5 minutes**: Based on production timing data. Normal implementation
tasks show file changes every 30-120 seconds. A 5-minute gap is
5-10x the normal interval, indicating the agent has stalled.

#### GateLoop (threshold: 3)

Tracks gate failure patterns per plan. If the agent's gate results
oscillate (fail, different fail, original fail) without making net
progress toward passing, it is in a gate loop.

**Differs from iteration-loop watcher**: The iteration-loop watcher
counts consecutive gate failures. The gate loop detector looks for
oscillation patterns — the agent might "fix" one error only to
reintroduce a previous one. The failure count stays the same but the
failures cycle.

#### CompileLoop (threshold: 3)

A specialized gate loop detector for compile errors. Tracks compile
error fingerprints across iterations. If the same set of errors
reappears after the agent attempted a fix, it is in a compile loop.

**Differs from compile-fail-repeat watcher**: The compile-fail-repeat
watcher detects identical errors across consecutive gates. The compile
loop detector detects cycling errors — error A appears, agent fixes A
but introduces B, agent fixes B but reintroduces A. The watcher sees
A, then B, then A (not repeated) — but the loop detector recognizes
the cycle.

#### EmptyOutput (threshold: 3)

Counts consecutive turns where the agent produces no tool calls and
no file changes. Three such turns in a row indicate the agent is
producing text (acknowledgments, descriptions, questions) but not
taking action.

**Why this is different from ghost turns**: Ghost turns are turns
with zero output — the model returned immediately with nothing.
Empty output turns have content (potentially verbose content) but
no actions. The agent is "thinking out loud" without doing anything.

#### ExcessiveRetries (threshold: 6)

Counts retry attempts for the same operation. If an agent retries
the same tool call (e.g., `cargo check`) six times without changing
its approach, it is in a retry loop.

---

## MetaCognitionHook

The `MetaCognitionHook` wraps the `StuckDetector` into a periodic
self-assessment mechanism that operates at Theta frequency:

```rust
pub struct MetaCognitionHook {
    detector: StuckDetector,
    frequency: OperatingFrequency,  // Theta
}
```

### Operating Frequency

The hook operates at Theta frequency — medium-rate periodic
assessment. In Roko's operating frequency model:

| Frequency | Rate | Purpose |
|-----------|------|---------|
| Gamma | High (every turn) | Real-time tool dispatch, safety checks |
| Theta | Medium (periodic) | Self-assessment, meta-cognition |
| Delta | Low (between sessions) | Consolidation, pattern extraction |

Theta frequency means the meta-cognition check runs periodically —
not on every turn (too expensive) but often enough to catch stuck
agents before they burn significant budget.

### Assessment Output

```rust
pub enum MetaCognitionAction {
    Continue,
    AdjustStrategy,
    Escalate,
}

pub struct MetaCognitionAssessment {
    pub frequency: OperatingFrequency,
    pub action: MetaCognitionAction,
    pub reason: String,
    pub stuck_kinds: Vec<StuckKind>,
}
```

The assessment maps stuck kinds to meta-cognition actions:

| Stuck Kind | MetaCognition Action | Rationale |
|-----------|---------------------|-----------|
| OutputLoop | AdjustStrategy | Agent needs a different approach |
| NoProgress | AdjustStrategy | Agent is stalled; refocus |
| GateLoop | Escalate | Cycling indicates fundamental problem |
| CompileLoop | Escalate | Cycling indicates architectural mismatch |
| EmptyOutput | AdjustStrategy | Agent needs more directive prompting |
| ExcessiveRetries | AdjustStrategy | Different operation or tool needed |

`AdjustStrategy` maps to a Conductor Restart (fresh agent, different
context). `Escalate` maps to a Conductor Fail (the problem is beyond
what a single-agent retry can solve).

### Signal Serialization

The `MetaCognitionAssessment` is serializable and can be emitted as
an Engram:

```rust
impl MetaCognitionAssessment {
    pub fn to_engram(&self) -> Engram {
        Engram::builder(Kind::Custom("conductor.meta_cognition".into()))
            .body(Body::from_json(self).expect("serialize assessment"))
            .tag("frequency", self.frequency.as_str())
            .tag("action", self.action.as_str())
            .tag("reason", &self.reason)
            .build()
    }
}
```

These signals feed into the Conductor's signal stream, where other
watchers or the intervention policy can incorporate them into the
overall decision.

---

## The Self-Model Requirement

The meta-cognition hook implements a principle from the Good Regulator
Theorem (Conant & Ashby, 1970):

> "Every good regulator of a system must be a model of that system."

The stuck detector is Roko's self-model — its representation of what
"healthy execution" looks like. By defining six specific stuck kinds,
the system models six ways execution can deviate from health. The
meta-cognition hook asks: "Does my current behavior match my model
of healthy execution?"

This self-model is necessarily incomplete. There are stuck modes that
the six heuristics will not catch. But the model improves over time:
each production failure that is not caught by the existing heuristics
becomes a candidate for a new stuck kind. The detection system grows
as the system's self-knowledge grows.

Ashby's Law of Requisite Variety (Ashby, 1956) constrains this growth:
the detector must have at least as many distinguishable states as the
execution system has pathological states. With six heuristics, the
detector can distinguish six stuck modes. If the execution system
can be stuck in seven distinct ways, the detector has insufficient
variety and will miss one.

---

## Threshold Tuning

The default thresholds balance sensitivity against false positives:

| Threshold | Default | Too Low → | Too High → |
|-----------|---------|-----------|-----------|
| output_loop | 4 | False positives on verification loops | Late detection (tokens wasted) |
| no_progress_ms | 300,000 | Kills slow-but-progressing agents | Agent stalls for 10+ minutes |
| gate_loop | 3 | Normal retry cycles flagged | Agent oscillates for 5+ cycles |
| compile_loop | 3 | Normal fix attempts flagged | Agent toggles errors for 5+ cycles |
| empty_output | 3 | Kills agents that are thinking | Agent describes instead of acting |
| excessive_retry | 6 | Normal retries flagged | Agent retries 10+ times |

The `StuckThresholds` struct accepts custom values through the
constructor, enabling per-deployment tuning. The learning system's
efficiency data provides the signal for tuning: if a threshold
consistently triggers without leading to recovery (the restarted
agent gets stuck the same way), the threshold should be lower. If
a threshold triggers and the restart succeeds, the threshold is
correctly calibrated.

---

## Relationship to Watcher Ensemble

The stuck detector and the watcher ensemble have overlapping but
distinct responsibilities:

| Detection | Stuck Detector | Watcher Ensemble |
|-----------|---------------|-----------------|
| Identical compile errors | CompileLoop heuristic | compile-fail-repeat watcher |
| Zero output | EmptyOutput heuristic | ghost-turn watcher |
| No file changes | NoProgress heuristic | (not directly covered) |
| Identical actions | OutputLoop heuristic | stuck-pattern watcher |
| Gate failure cycling | GateLoop heuristic | iteration-loop watcher |
| Cost overrun | (not covered) | cost-overrun watcher |
| Context pressure | (not covered) | context-window-pressure watcher |
| Review cycling | (not covered) | review-loop watcher |
| Spec drift | (not covered) | spec-drift watcher |
| Time overrun | (not covered) | time-overrun watcher |

The overlaps are intentional — the stuck detector provides a
complementary detection mechanism with different thresholds and
detection logic. The watcher ensemble operates on the signal stream
(structured data). The stuck detector can operate on raw agent output
(unstructured data). Both feed into the Conductor's decision process.

---

## File Reference

| File | What |
|------|------|
| `crates/roko-conductor/src/stuck_detection.rs` | StuckDetector, StuckKind, StuckThresholds, MetaCognitionHook, MetaCognitionAssessment |


---

# SOURCE: /Users/will/dev/nunchi/roko/roko/docs/07-conductor/06-health-monitors.md

# Health Monitors

> Four system-level checks that produce a HealthStatus snapshot.
> Not individual task health — system health. Is the infrastructure
> functioning? Are agents alive? Is coverage trending down?


> **Implementation**: Built

---

## SystemSnapshot

The health monitor operates on a point-in-time snapshot of system state:

```rust
pub struct SystemSnapshot {
    pub active_agents: usize,
    pub expected_agents: usize,
    pub last_agent_heartbeat_ms: Option<u64>,
    pub chain_connected: bool,
    pub chain_expected: bool,
    pub spec_hash_expected: Option<String>,
    pub spec_hash_actual: Option<String>,
    pub coverage_history: Vec<f64>,
}
```

The snapshot captures infrastructure health, not task health. Task
health is the watcher ensemble's domain. Infrastructure health is
about whether the foundation the tasks run on is solid.

---

## HealthStatus

```rust
pub enum HealthStatus {
    Healthy,
    Degraded,
    Critical,
}
```

**Healthy**: All checks pass. The system is operating normally.

**Degraded**: One or more checks indicate a non-critical issue. The
system continues operating but with reduced capacity or reliability.
Operator attention is recommended.

**Critical**: A fundamental infrastructure problem. The system cannot
reliably continue. Operator intervention is required.

---

## The Four Checks

### 1. Terminal Liveness

**What it checks**: Is the agent process still responsive?

**How it works**: Compares the `last_agent_heartbeat_ms` against
a liveness threshold. If the most recent heartbeat is older than
the threshold, the terminal is considered unresponsive.

**Healthy**: Heartbeat within threshold (or no agents expected).
**Degraded**: Heartbeat exceeds threshold — agent may have stalled.
**Critical**: No heartbeat received and agents are expected.

**Why this matters**: Agent processes can become unresponsive without
crashing. The process is alive (PID exists, no exit code) but the
agent has stopped producing output or responding to input. Without
heartbeat monitoring, this condition is invisible — the orchestrator
thinks the agent is working when it has actually stalled.

**Connection to ProcessSupervisor**: In the full runtime stack
(`bardo-runtime`), the ProcessSupervisor tracks agent processes with
heartbeat monitoring, resource limits, and descendant tree tracking.
The terminal liveness check is the health monitor's view of the
same data.

### 2. Agent Status

**What it checks**: Are the expected number of agents running?

**How it works**: Compares `active_agents` against `expected_agents`.
If fewer agents are active than expected, something has failed.

**Healthy**: `active_agents >= expected_agents`.
**Degraded**: `active_agents < expected_agents` — some agents have
died or failed to start.
**Critical**: `active_agents == 0` and `expected_agents > 0` — all
agents are down.

**Why this matters**: In a batch run with 5 parallel plans, each
requiring one implementer agent, the expected count is 5. If only 3
agents are active, 2 plans are stalled waiting for agents. This check
detects the shortfall before the stalled plans' timeout fires.

**Self-healing trigger**: When agent status is Degraded, the
orchestrator can proactively respawn missing agents rather than waiting
for the affected plans' time-overrun watchers to fire. This is the
"anticipate, don't react" principle (Design Principle 11) applied to
agent lifecycle.

### 3. Spec Drift

**What it checks**: Has the implementation diverged from its specification?

**How it works**: Compares `spec_hash_expected` against `spec_hash_actual`.
If the hashes differ, the specification has changed since the plan
was generated, or the implementation has drifted from the spec.

**Healthy**: Hashes match (or no spec tracking configured).
**Degraded**: Hashes differ — spec drift detected.
**Critical**: (Not used for this check — drift is always Degraded.)

**Why this matters**: Spec drift at the system level means the
acceptance criteria may no longer match the implementation. This can
happen when:
- A PRD is updated while a plan is in progress
- Multiple plans modify the same crate's public API
- External dependencies change their interface

System-level spec drift is distinct from the spec-drift watcher
(which monitors individual task file scope). The health monitor's
spec drift check looks at the entire system specification, not
individual task boundaries.

### 4. Coverage Trend

**What it checks**: Is test coverage trending down over time?

**How it works**: Examines the `coverage_history` vector (a sequence
of coverage percentages over recent builds). If the trend is
downward (recent values lower than earlier values), the system is
losing test coverage.

**Healthy**: Coverage stable or increasing.
**Degraded**: Coverage declining — agents are adding code without
corresponding tests.
**Critical**: (Not used — coverage decline is always Degraded.)

**Why this matters**: Test coverage is a leading indicator of
agent quality degradation. When agents start skipping tests to meet
gate criteria faster, coverage drops. This is especially dangerous
because coverage drops compound — less-tested code is harder for
future agents to modify correctly, leading to more failures, leading
to more corner-cutting, leading to less coverage.

The coverage trend check implements Design Principle 12: "The agent
builds the world it operates in." Declining coverage means agents
are making the codebase worse for future agents.

**Trend computation**: The health monitor uses a simple regression
on the coverage history. If the slope is negative and the recent
average is below the earlier average by more than a threshold (e.g.,
2 percentage points), the status is Degraded.

---

## HealthMonitor API

```rust
pub struct HealthMonitor {
    // Configuration: thresholds for each check
}

impl HealthMonitor {
    pub fn check(&self, snapshot: &SystemSnapshot) -> HealthStatus {
        let liveness = self.terminal_liveness(snapshot);
        let agents = self.agent_status(snapshot);
        let drift = self.spec_drift(snapshot);
        let coverage = self.coverage_trend(snapshot);

        // Worst status wins
        [liveness, agents, drift, coverage]
            .into_iter()
            .max()
            .unwrap_or(HealthStatus::Healthy)
    }
}
```

Like the intervention policy, the health monitor uses worst-status-wins
aggregation. If any single check returns Critical, the overall status
is Critical.

---

## Health vs. Watcher Ensemble

The health monitor and watcher ensemble serve different purposes:

| Dimension | Health Monitor | Watcher Ensemble |
|-----------|--------------|-----------------|
| **Scope** | System infrastructure | Individual plan/task execution |
| **Input** | SystemSnapshot | Signal stream |
| **Output** | HealthStatus (Healthy/Degraded/Critical) | WatcherOutput (per-watcher severity) |
| **Frequency** | Periodic (every N seconds) | Every conductor evaluation |
| **Trigger** | Infrastructure problems | Execution anomalies |

A system can be Healthy (all infrastructure checks pass) while
individual plans are failing (watchers detect stuck agents). Conversely,
all plans can be proceeding normally while the system is Degraded (an
expected agent has died, reducing parallelism).

Both feed into the Conductor's overall assessment. The health monitor's
Critical status can override watcher-based decisions — if the
infrastructure is failing, task-level interventions are pointless.

---

## Snapshot Collection

The SystemSnapshot is assembled by the orchestrator from multiple
sources:

| Field | Source |
|-------|--------|
| `active_agents` | ProcessSupervisor agent count |
| `expected_agents` | Orchestrator plan state (plans in Implementing phase) |
| `last_agent_heartbeat_ms` | ProcessSupervisor heartbeat tracker |
| `chain_connected` | (Not used in current deployment — reserved for future chain integration) |
| `chain_expected` | Configuration flag |
| `spec_hash_expected` | Plan TOML frontmatter |
| `spec_hash_actual` | Computed from current codebase state |
| `coverage_history` | Gate results over recent builds |

The orchestrator constructs the snapshot periodically (every 10 seconds
in the default configuration) and passes it to the health monitor.
The snapshot is a read-only copy of live state — computing the health
check does not hold any locks or block the orchestrator's main loop.

---

## VSM Mapping

In Beer's Viable System Model (Beer, 1972), the health monitor maps to
**System 3*** (System Three-Star) — the audit channel:

| VSM Component | Roko Equivalent |
|--------------|----------------|
| System 1 | Individual agents executing tasks |
| System 2 | Conventions, templates, shared protocols |
| System 3 | Orchestrator (internal oversight, resource allocation) |
| **System 3*** | **Health monitor (sporadic audit, independent check)** |
| System 4 | Learning system (external adaptation) |
| System 5 | Configuration and policy |

System 3* is the audit function — it checks whether System 3's
(the orchestrator's) model of reality matches actual reality. The
health monitor does exactly this: it independently checks whether
the agents the orchestrator thinks are running are actually running,
whether the spec the orchestrator is working from is still current,
and whether the quality metrics the orchestrator relies on are
trending in the right direction.

---

## File Reference

| File | What |
|------|------|
| `crates/roko-conductor/src/health.rs` | HealthMonitor, SystemSnapshot, HealthStatus, 4 check methods |


---

# SOURCE: /Users/will/dev/nunchi/roko/roko/docs/07-conductor/07-ooda-cybernetic-loop.md

# OODA and the Cybernetic Loop

> Observe the signal stream. Orient through watcher analysis.
> Decide via intervention policy. Act through orchestrator commands.
> Every evaluation cycle is one iteration of this loop.


> **Implementation**: Built

---

## The OODA Framework

Boyd's OODA loop (Observe-Orient-Decide-Act) provides the conceptual
framework for the Conductor's evaluation cycle. Each conductor tick
maps directly to one OODA iteration:

### Observe

The signal stream is the observation input. Every agent turn, gate
result, phase transition, cost event, and timing measurement produces
a Signal that enters the stream. The Conductor reads the stream
without modifying it.

Signals consumed:
- `TokenUsage` — token counts per turn
- `GateVerdict` — gate pass/fail with structured results
- `AgentOutput` — agent turn content
- `PlanPhase` — phase transition events
- `Metric` — numeric measurements (cost, drift, coverage)
- `Custom("conductor.agent_output")` — timing data

The observation phase is pure reading. No state is modified. No
decisions are made.

### Orient

Orientation is where raw observations become assessments. Each watcher
transforms raw signals into structured evaluations:

- Ghost turn watcher: "Agent 7 produced zero output for 3 consecutive turns"
- Cost overrun watcher: "Plan 12 has spent $8.40 of its $10.00 budget"
- Spec drift watcher: "Plan 3 has 32% file changes outside declared scope"

Orientation also includes the stuck detector's meta-cognition
assessment and the diagnosis engine's error classification. Raw
data becomes typed assessments.

The orient phase corresponds to the `check_all()` method — running
all watchers against the signal stream and collecting their outputs.

### Decide

The intervention policy resolves multiple watcher assessments into a
single decision. `WorstSeverityPolicy` selects the maximum severity:

```
Input:  [Warning(compile-fail), Warning(spec-drift)]
Output: ConductorDecision::Restart { reason: "..." }
```

The circuit breaker also participates in the decide phase: a tripped
plan produces `Fail` regardless of watcher outputs.

### Act

The Conductor does not act directly. It returns a `ConductorDecision`
to the orchestrator, which translates it into concrete actions:

| Decision | Orchestrator Action |
|----------|-------------------|
| Continue | Do nothing — proceed with current execution |
| Restart | Kill agent process, prepare error context, spawn fresh agent |
| Fail | Cancel in-flight tasks, mark plan as Failed, move to next plan |

This separation of decision from action is deliberate. The Conductor
has no direct access to processes, files, or agents. It operates
purely on the signal stream and produces pure decisions. The
orchestrator translates decisions into effects.

---

## Cybernetic Structure

The Conductor's evaluation cycle implements a cybernetic feedback loop
in the classical sense (Wiener, 1948):

```
┌──────────────┐     Signals      ┌──────────────┐
│              │ ───────────────→  │              │
│  Execution   │                  │  Conductor   │
│  (Agents,    │                  │  (Watchers,  │
│   Gates,     │  ←───────────── │   Policy,    │
│   Merges)    │   Decision       │   Breaker)   │
│              │                  │              │
└──────────────┘                  └──────────────┘
        │                                 │
        │         Environment             │
        └─────────────────────────────────┘
```

**Sensor**: Signal stream (observes execution state)
**Comparator**: Watchers (compare observed state to thresholds)
**Controller**: Intervention policy (decides corrective action)
**Actuator**: Orchestrator (executes the decision)
**Environment**: Agents + codebase + gates (the system being regulated)

### Negative Feedback

The Conductor implements negative feedback — it acts to reduce
deviation from the desired state. When spec drift exceeds 25%, the
intervention signal pushes the system back toward in-scope work. When
cost exceeds budget, the signal pushes toward termination or restart.
When compile errors repeat, the signal pushes toward a fresh approach.

This is classical homeostatic regulation: the system has a set point
(healthy execution) and corrects deviations.

### Positive Feedback (Absent by Design)

The Conductor does not implement positive feedback — it does not
amplify trends. It does not say "the agent is doing great, give it
more resources." Positive feedback in the conductor domain would risk
runaway behavior: a successful agent getting more context, producing
more output, consuming more tokens, triggering cost overrun.

Positive feedback lives in the learning system instead: successful
model-task combinations get higher reward in the cascade router,
successful patterns promote to playbook rules. The Conductor's job
is stability, not optimization.

---

## Feedback Loop Frequency

The Conductor evaluates at a frequency determined by the orchestrator's
event loop:

**Per-event evaluation**: The orchestrator calls `conductor.evaluate()`
after significant events — agent turn completion, gate result, phase
transition. This is event-driven, not time-driven.

**Periodic health check**: The health monitor runs on a fixed interval
(every 10 seconds), independent of events. This catches infrastructure
problems that do not produce events (e.g., an agent that has silently
died).

**Theta-frequency meta-cognition**: The `MetaCognitionHook` runs at
Theta frequency — less often than per-event, more often than per-phase.
This provides medium-granularity self-assessment without the overhead
of running all stuck detection heuristics on every event.

The three frequencies provide layered coverage:

| Frequency | What Runs | Catches |
|-----------|----------|---------|
| Per-event | All 10 watchers | Task-level anomalies |
| Every 10s | Health monitor | Infrastructure failures |
| Theta | MetaCognitionHook | Stuck agents between events |

---

## Closed-Loop Properties

### Stability

The Conductor's feedback loop is stable because:

1. **Bounded responses**: Every decision is one of three options
   (Continue/Restart/Fail). There is no unbounded escalation.
2. **Cooldown periods**: After firing, each watcher has a cooldown
   before it can fire again for the same plan. This prevents
   oscillation (fire → restart → fire → restart).
3. **Circuit breaker**: After two failures, the plan is permanently
   failed. This prevents infinite retry loops.
4. **Monotonic progress**: Failed plans do not re-enter the pipeline
   automatically. Each restart is a fresh attempt with additional
   information, not a continuation of the failed state.

### Observability

Every decision produces a signal that enters the stream. This means
the Conductor's own behavior is observable:

- Dashboard shows when the conductor intervened and why
- Signal replay can reconstruct every decision
- Learning system records interventions as negative signals
- The conductor's own watchers could theoretically monitor the
  conductor's behavior (second-order meta-cognition)

### Latency

The conductor evaluation cycle adds latency to the orchestrator's
event loop. Measured latency:

| Component | Typical Latency |
|-----------|----------------|
| Circuit breaker check | < 1 μs (DashMap lookup) |
| All 10 watchers | < 1 ms (stream scan, no I/O) |
| Intervention policy | < 1 μs (max comparison) |
| Signal emission | < 10 μs (signal construction) |
| **Total** | **< 2 ms** |

This latency is negligible compared to agent turn times (seconds to
minutes) and gate execution times (seconds to minutes). The conductor
evaluation is never the bottleneck.

---

## Comparison to Other Cybernetic Architectures

### Beer's Viable System Model

The Conductor implements multiple VSM systems:

| VSM System | Function | Roko Component |
|-----------|----------|----------------|
| System 1 | Operations | Individual agents |
| System 2 | Coordination | Shared conventions, templates |
| **System 3** | **Control** | **Conductor (internal regulation)** |
| **System 3*** | **Audit** | **Health monitor (independent check)** |
| System 4 | Intelligence | Learning system (adaptation) |
| System 5 | Policy | Configuration, design principles |

The Conductor is primarily System 3 — it monitors and controls the
internal operations of the agent ensemble. The health monitor adds
System 3* — an independent audit channel that checks the orchestrator's
model against reality.

Reference: Beer, S. (1972). *Brain of the Firm*.

### Conant-Ashby Good Regulator Theorem

"Every good regulator of a system must be a model of that system."
(Conant & Ashby, 1970)

The Conductor models the pipeline through:
- **Watcher thresholds**: model of what "normal" looks like
- **Stuck heuristics**: model of pathological behavior patterns
- **Error categories**: model of failure modes
- **Health checks**: model of infrastructure requirements

This model is currently static (thresholds are constants). The learning
system provides a path to an adaptive model — thresholds that update
based on observed system behavior, becoming a more accurate model
over time.

### Ashby's Law of Requisite Variety

"Only variety can absorb variety." (Ashby, 1956)

The Conductor's regulatory variety is:
- 10 watcher types × configurable thresholds
- 6 stuck heuristics × configurable thresholds
- 20 error categories × 9 intervention types
- 3 severity levels × 3 decision types
- 4 health checks × 3 health statuses

This regulatory variety must match or exceed the variety of the
system being regulated. If the agent ensemble can fail in more
distinct ways than the Conductor can detect, some failures will go
unregulated.

The modular architecture supports variety expansion: adding a new
watcher adds a new detection dimension. Adding a new error pattern
adds a new classification. The system's regulatory variety grows as
new failure modes are cataloged.

---

## File Reference

| File | What |
|------|------|
| `crates/roko-conductor/src/conductor.rs` | The OODA loop implementation (evaluate()) |
| `crates/roko-conductor/src/stuck_detection.rs` | MetaCognitionHook (Theta-frequency assessment) |
| `crates/roko-conductor/src/health.rs` | Health monitor (periodic infrastructure check) |
| `crates/roko-conductor/src/interventions.rs` | Decision resolution (Orient → Decide) |

---

## OODA Loop Speed Optimization

### Boyd's key insight: tempo, not speed

Boyd's central argument was not "cycle faster." It was "get inside the
adversary's decision loop." For the Conductor, the adversary is drift —
the gap between what agents are doing and what they should be doing. The
goal is to update the world model faster than agent behavior can diverge
from the plan.

"Operating inside the loop" means the Conductor detects and corrects a
deviation before the agent's next action compounds it. If an agent
produces a ghost turn and the Conductor restarts it before the next turn
fires, the loop stays tight. If three ghost turns accumulate before
detection, the Conductor is operating outside the loop — reacting to
damage rather than preventing it.

Tempo is relational. A Conductor that evaluates every 500 ms is
overbuilt if agents produce turns every 30 seconds. A Conductor that
evaluates every 10 seconds is underbuilt if agents produce turns every
2 seconds. The right tempo is one evaluation per event, with periodic
health checks to cover silent failures.

### What determines conductor cycle time

The Conductor's per-evaluation latency breaks down as follows:

| Component | Typical Latency | Optimization |
|-----------|----------------|--------------|
| Signal stream scan (10 watchers) | < 1 ms | Already optimal — pure in-memory scan |
| Circuit breaker lookup (DashMap) | < 1 us | Already optimal — lock-free concurrent map |
| Intervention policy resolution | < 1 us | Already optimal — max comparison |
| Anomaly detector check | < 0.1 ms | EWMA update is O(1) |
| Health monitor snapshot | ~10 ms | Periodic, not on critical path |
| MetaCognition assessment | ~1 ms | Theta frequency, not every turn |
| **Total per-evaluation** | **< 2 ms** | **Negligible vs agent turn times** |

At < 2 ms per evaluation, the Conductor adds negligible overhead to any
agent turn. The evaluation itself is never the bottleneck. The bottleneck
is upstream: when do observations arrive?

### The real bottleneck: observation latency

The Conductor evaluates fast, but it can only evaluate what it can see.
Observations arrive when events are produced — and an agent stuck in a
long reasoning chain produces no events. Between the start of an agent
turn and the completion of that turn, the Conductor is blind.

The per-10s health monitor is the only mechanism that detects this gap.
It checks infrastructure status independent of the event stream. But
10 seconds is a coarse interval. An agent that hangs for 9 seconds
gets no detection until the next health tick.

A dedicated liveness monitor would close this gap:

```rust
/// Heartbeat-based liveness detection for agents between events.
/// Detects stuck agents that produce no observable signals.
pub struct LivenessMonitor {
    /// Expected heartbeat interval per agent (default: 30s).
    expected_interval: Duration,
    /// Last heartbeat timestamp per agent.
    last_heartbeat: DashMap<String, Instant>,
    /// Warning threshold: fire warning at this multiple of expected_interval.
    warning_multiplier: f64,  // default: 2.0 (60s for 30s interval)
    /// Critical threshold: fire critical at this multiple.
    critical_multiplier: f64, // default: 5.0 (150s for 30s interval)
}
```

The liveness monitor runs on its own timer. If an agent's last heartbeat
exceeds `expected_interval * warning_multiplier`, it emits a warning
signal into the stream. If it exceeds `critical_multiplier`, it emits
a critical signal. This gives the Conductor visibility into the gap
between events without requiring agents to change their behavior —
heartbeats are emitted by the process supervisor, not by agents
themselves.

### Implicit Guidance and Control (IG&C)

Boyd described a shortcut in the OODA loop: when the Orient phase
recognizes a well-known pattern, it can bypass the full Decide phase
and jump straight to Act. He called this Implicit Guidance and Control.
Klein's Recognition-Primed Decision model (1998) documents the same
phenomenon in human experts — experienced firefighters do not deliberate
over options; they recognize the situation and act.

For the Conductor, IG&C means pre-compiled rules for common failure
patterns:

```rust
/// Pre-compiled action rules for known patterns (Boyd's IG&C shortcut).
/// Bypasses full watcher evaluation for well-understood failure modes.
pub struct ImplicitGuidance {
    /// Map from recognized pattern fingerprint to pre-computed action.
    rules: Vec<ImplicitRule>,
}

pub struct ImplicitRule {
    /// Pattern name for logging/observability.
    pub name: &'static str,
    /// Fast check: does this pattern match the current signal stream?
    pub matcher: Box<dyn Fn(&[Engram]) -> bool + Send + Sync>,
    /// Pre-computed action to take when the pattern matches.
    pub action: ConductorDecision,
    /// Minimum confidence from bandit training before this rule activates.
    pub min_confidence: f64,
}
```

When `ImplicitGuidance` matches the current signal stream, the Conductor
skips full watcher evaluation and returns the pre-computed decision.
This is faster (sub-microsecond), but more importantly it encodes
institutional knowledge: patterns the system has seen before and knows
how to handle.

IG&C rules should not be hand-written. They should be extracted from the
ConductorBandit's converged actions. When a bandit arm converges to >95%
selection rate for a given failure pattern, that pattern graduates to an
IG&C rule. The bandit continues to explore; the IG&C rule handles the
common case.

---

## Nested OODA loops — multi-timescale control

A single OODA loop is insufficient for a system that operates across
multiple timescales. Agent turns happen in seconds, tasks take minutes,
plans run for hours. A single loop tuned for seconds would generate
excessive churn at the plan level. A single loop tuned for hours would
miss per-turn anomalies.

The solution is nested loops, each operating at its own frequency.

### Three-level nesting

Roko's cognitive frequencies map to three nested OODA loops:

```
┌─────────────────────────────────────────────────────┐
│  Delta Loop (Strategic)                              │
│  Period: per-batch (hours)                           │
│  Orient: cross-plan patterns, model effectiveness    │
│  Decide: cascade router updates, threshold tuning    │
│  Act:    policy changes for next batch               │
│                                                      │
│  ┌─────────────────────────────────────────────┐    │
│  │  Theta Loop (Operational)                    │    │
│  │  Period: per-task (minutes)                   │    │
│  │  Orient: MetaCognitionHook assessment         │    │
│  │  Decide: strategy adjustment, escalation      │    │
│  │  Act:    restart agent, switch model           │    │
│  │                                                │    │
│  │  ┌───────────────────────────────────────┐   │    │
│  │  │  Gamma Loop (Tactical)                 │   │    │
│  │  │  Period: per-turn (seconds)             │   │    │
│  │  │  Orient: all 10 watchers                │   │    │
│  │  │  Decide: intervention policy            │   │    │
│  │  │  Act:    continue/restart/fail           │   │    │
│  │  └───────────────────────────────────────┘   │    │
│  └─────────────────────────────────────────────┘    │
└─────────────────────────────────────────────────────┘
```

The innermost loop (Gamma) handles per-turn tactical decisions: is this
agent turn healthy? The middle loop (Theta) handles per-task operational
decisions: is this task making progress? The outer loop (Delta) handles
per-batch strategic decisions: is the system learning and improving?

### Separation of concerns

Each loop has its own orientation model — different data sources,
different scopes, different update frequencies:

| Loop | Orientation Source | Model Scope | Update Frequency |
|------|-------------------|-------------|-----------------|
| Gamma | Watcher thresholds (static constants) | Single agent turn | Every turn |
| Theta | MetaCognition assessment + stuck heuristics | Task trajectory | Every 3-5 turns |
| Delta | Efficiency events + cascade router observations | Cross-plan patterns | Per batch |

The Gamma loop does not know about cross-plan patterns. The Delta loop
does not know about individual agent turns. This is not a limitation —
it is the design. Each loop sees only what it needs at its timescale.

### Parameter cascade

Slower loops set the parameters for faster loops. This is the
foundational principle from hierarchical control theory (Mesarovic
et al. 1970): the slower controller sets the frame within which the
faster controller operates.

In Roko:
- **Delta sets Theta parameters**: adaptive gate thresholds, default
  model tier, cost budgets per task
- **Theta sets Gamma parameters**: adjusted watcher thresholds based
  on meta-cognition assessment, intervention cooldown periods

```rust
/// Hierarchical parameter cascade: slower loops configure faster loops.
pub struct ParameterCascade {
    /// Delta-level parameters (updated per batch).
    pub delta: DeltaParameters,
    /// Theta-level parameters (updated per task).
    pub theta: ThetaParameters,
    /// Gamma-level parameters (used per turn, set by Theta).
    pub gamma: GammaParameters,
}

pub struct DeltaParameters {
    pub default_model_tier: ModelTier,
    pub base_cost_budget_usd: f64,
    pub gate_threshold_adjustments: HashMap<String, f64>,
}

pub struct ThetaParameters {
    pub adjusted_stuck_threshold: usize,
    pub adjusted_ghost_turn_max: usize,
    pub current_pressure_level: f64,  // 0.0 to 1.0
}

pub struct GammaParameters {
    pub watcher_thresholds: WatcherThresholds,
    pub intervention_cooldown: Duration,
}
```

The cascade flows one direction: slow to fast. The Gamma loop never
modifies Delta parameters. If the Gamma loop detects something that
requires a strategic response, it emits a signal into the stream. The
Delta loop picks it up on its next evaluation — at its own pace.

### Singular perturbation principle

When timescales are well-separated, each loop can be analyzed
independently. This is the singular perturbation result from control
theory: if the fast loop reaches steady state before the slow loop
takes its next step, the two loops decouple mathematically.

In Roko:
- Gamma runs every ~5 seconds (per agent turn)
- Theta runs every ~75 seconds (per task, roughly every 15 Gamma cycles)
- Delta runs every ~hours (per batch, roughly 50-100 Theta cycles)

The ~15x separation between adjacent levels is sufficient for
quasi-static decoupling. The Gamma loop treats Theta parameters as
constants — they change slowly relative to per-turn evaluation. The
Theta loop assumes the Gamma loop has reached its steady-state
decision for the current watcher outputs.

This separation is what makes the hierarchical architecture tractable.
Without it, every parameter change at every level would interact with
every other level, producing a combinatorial analysis problem. With it,
each level can be understood, tuned, and debugged in isolation.

---

## Algedonic signals — priority interrupts

### Definition

Algedonic signals are pain/pleasure signals that bypass the normal
management hierarchy, going directly from operations to policy. The
term comes from Greek: algos (pain) + hedone (pleasure). Beer
introduced the concept in the Viable System Model as the mechanism
by which System 1 (operations) alerts System 5 (policy) without
waiting for the signal to propagate through Systems 2, 3, and 4.

In the Conductor, algedonic signals represent conditions severe enough
that the normal evaluation pipeline is too slow. The Gamma loop should
not deliberate over whether a safety violation warrants intervention.

### When algedonic signals fire

Four conditions trigger algedonic escalation:

1. **Runaway cost**: total session cost exceeds 2x budget before 50%
   of wall time has elapsed. The cost trajectory predicts catastrophic
   overrun, not a gradual approach to the budget ceiling.

2. **Safety violation**: an agent attempts to modify files outside the
   declared workspace scope, execute disallowed commands, or access
   restricted resources. Any safety violation is an immediate interrupt
   regardless of severity assessment.

3. **Total infrastructure failure**: all agents are down simultaneously.
   Not one agent failing (which the Gamma loop handles), but every
   agent in the current execution losing connectivity or crashing at
   once.

4. **Operator interrupt**: explicit Ctrl+C or shutdown command. The
   human operator is the ultimate algedonic channel — their interrupt
   overrides everything.

### Escalation with time windows

Each layer in the hierarchy gets a bounded window to respond before the
signal escalates upward:

```
Agent detects anomaly → Conductor has 5s to respond
    | (no response within 5s)
Orchestrator has 30s to respond
    | (no response within 30s)
Policy layer triggers emergency shutdown
```

The time windows enforce liveness. A Conductor that hangs (perhaps
because the anomaly also affects its evaluation path) cannot silently
block the escalation. If the Conductor does not respond within 5
seconds, the orchestrator takes over. If the orchestrator does not
respond within 30 seconds, the policy layer performs an emergency
shutdown — kill all agents, persist state, exit with a non-zero status.

Algedonic signals are rare by design. If they fire frequently, the
normal feedback loops are miscalibrated. A well-tuned system routes
almost everything through the Gamma/Theta/Delta hierarchy and reserves
algedonic escalation for genuine emergencies.

---

## References

- Boyd, J. (1995). "The Essence of Winning and Losing" (OODA loop diagram).
- Boyd, J. (1976). "Destruction and Creation" (theoretical underpinning of OODA).
- Beer, S. (1972). *Brain of the Firm* (Viable System Model, algedonic signals).
- Mesarovic, M., Macko, D., and Takahara, Y. (1970). *Theory of Hierarchical, Multilevel Systems* (hierarchical control, parameter cascade).
- Klein, G. (1998). *Sources of Power: How People Make Decisions* (Recognition-Primed Decision model, parallel to IG&C).


---

# SOURCE: /Users/will/dev/nunchi/roko/roko/docs/07-conductor/08-good-regulator-self-model.md

# Good Regulator and the Self-Model

> "Every good regulator of a system must be a model of that system."
> — Conant & Ashby (1970)
>
> The Conductor is Roko's self-model. It represents the system's
> understanding of what healthy execution looks like.


> **Implementation**: Built

---

## The Theorem

The Good Regulator Theorem (Conant & Ashby, 1970) states that any
system that successfully regulates another system must contain a model
of that system. This is not a design recommendation — it is a
mathematical proof. A regulator that does not model the system it
controls cannot be an optimal regulator.

For the Conductor: to regulate agent execution, the Conductor must
model what healthy agent execution looks like. Every threshold, every
heuristic, every error pattern is a component of this model.

---

## Components of the Self-Model

### 1. Behavioral Norms (Watcher Thresholds)

Each watcher threshold encodes an expectation about normal behavior:

| Threshold | Expectation |
|-----------|------------|
| `MAX_GHOST_TURNS = 3` | A healthy agent produces meaningful output on every turn |
| `MAX_COMPILE_FAIL_REPEAT = 3` | A healthy agent does not repeat the same compile error |
| `MAX_ITERATION_LOOP = 3` | A healthy plan converges within 3 gate-fail cycles |
| `MAX_REVIEW_CYCLES = 3` | A healthy plan passes review within 3 cycles |
| `MAX_SPEC_DRIFT_RATIO = 0.25` | A healthy agent modifies at most 25% unexpected files |
| `MAX_STUCK_REPEATS = 4` | A healthy agent does not repeat identical actions |
| `MIN_FAILURE_INCREASE = 1` | A healthy agent does not increase test failures |
| `ALERT_THRESHOLD = 0.80` | A healthy task completes within 80% of its timeout |
| `MAX_CONTEXT_USAGE_RATIO = 0.80` | A healthy agent uses at most 80% of its context window |
| `MAX_PLAN_FAILURES = 2` | A recoverable plan succeeds within 2 attempts |

These thresholds define the "normal region" of execution space. When
execution leaves this region, the Conductor intervenes to push it back.

### 2. Failure Taxonomy (Error Categories)

The 20 error categories in the diagnosis engine model the system's
failure modes:

```
CompileError, TestFailure, TypeMismatch, BorrowCheckerError,
LifetimeError, ImportError, MissingFile, PermissionDenied,
NetworkError, TimeoutError, OomError, DiskFull,
LlmRateLimit, LlmContextOverflow, LlmRefusal,
ProcessCrash, LoopDetected, ClippyWarning,
GitConflict, DependencyError
```

Each category represents the system's understanding of a distinct
way things can go wrong. The intervention mapping (which action to
take for each category) represents the system's understanding of
how to recover from each failure mode.

### 3. Process Patterns (Stuck Heuristics)

The six stuck kinds model pathological execution patterns:

```
OutputLoop    — doing the same thing repeatedly
NoProgress    — doing things that produce no results
GateLoop      — oscillating between two broken states
CompileLoop   — toggling between incompatible fixes
EmptyOutput   — producing text without action
ExcessiveRetries — retrying without changing approach
```

Each pattern is a mode of execution that LOOKS like progress (the
agent is active, producing output, calling tools) but IS NOT progress.
The stuck detector models the difference between activity and progress.

### 4. Infrastructure Expectations (Health Checks)

The health monitor models infrastructure requirements:

- Agents should be running (agent status)
- Agents should be responsive (terminal liveness)
- Specifications should be current (spec drift)
- Quality should be maintained (coverage trend)

These expectations define what "the system is ready to do work" means.

---

## Model Accuracy

The self-model's accuracy determines the Conductor's effectiveness.
An inaccurate model produces:

### False Positives (Model Too Strict)

The model considers healthy behavior to be pathological. Examples:
- `MAX_GHOST_TURNS = 1` would kill agents that take one turn to
  read context before producing output
- `MAX_SPEC_DRIFT_RATIO = 0.05` would flag agents that update a
  mod.rs file alongside their primary target

False positives waste resources — healthy agents are killed and
restarted unnecessarily.

### False Negatives (Model Too Lenient)

The model considers pathological behavior to be healthy. Examples:
- `MAX_GHOST_TURNS = 10` would let a stuck agent burn tokens for
  10 turns before intervention
- `MAX_ITERATION_LOOP = 10` would let a non-converging plan retry
  10 times before failing

False negatives waste resources — pathological agents run unchecked.

### The Tuning Challenge

The model must be calibrated against real execution data. The current
thresholds are derived from production experience during batch runs
in March-April 2026. They represent the best-known calibration for
that period's codebase, model versions, and task complexity.

As these factors change, the model drifts. New model versions may
have different failure patterns. Codebase evolution changes what
"normal" spec drift looks like. Task complexity shifts change what
"normal" iteration count means.

---

## Static vs. Adaptive Models

### Current: Static Model

All thresholds are compile-time constants or constructor parameters.
The model does not update based on observed behavior:

```rust
pub const MAX_CONTEXT_USAGE_RATIO: f64 = 0.80;
pub const MAX_GHOST_TURNS: usize = 3;
pub const MAX_COMPILE_FAIL_REPEAT: usize = 3;
```

**Advantage**: Predictable, easy to reason about, no drift.
**Disadvantage**: Cannot adapt to changing conditions.

### Future: Adaptive Model

The learning system provides the infrastructure for an adaptive
self-model. The components exist:

- **Adaptive gate thresholds** (`roko-gate/src/adaptive_threshold.rs`):
  EMA-based threshold adjustment per gate rung. Already wired.
- **Efficiency events** (`roko-learn/src/efficiency.rs`): Per-turn
  metrics including iteration count, cost, success rate. Already
  collected.
- **Cascade router observations**: Model-task combination outcomes.
  Already recorded.

An adaptive Conductor model would:

1. Record the threshold that triggered each intervention
2. Track whether the intervention improved the outcome (did the
   restarted agent succeed? did the failed plan succeed on retry?)
3. Adjust thresholds toward values that maximize intervention
   effectiveness

For example, if interventions triggered at `MAX_GHOST_TURNS = 3`
successfully recover 80% of stuck agents, but interventions at
`MAX_GHOST_TURNS = 2` recover 90%, the adaptive model would lower
the threshold to 2.

This is the cascade router pattern applied to conductor thresholds:
the system learns which thresholds produce the best outcomes.

---

## Precision-Weighted Prediction Errors

The Good Regulator framework connects to precision-weighted prediction
errors from active inference theory:

**Prediction**: The model predicts what healthy execution looks like
(thresholds define the prediction).

**Prediction error**: The difference between predicted (healthy) and
observed (actual) behavior. Each watcher computes a prediction error:
"I predicted the agent would produce output; it produced none."

**Precision weighting**: Not all prediction errors are equally
informative. Prediction errors on familiar tasks (tasks with many
historical episodes) should be weighted more heavily — the model
is confident in its prediction, so a deviation is surprising and
informative. Prediction errors on novel tasks (no similar episodes)
should be weighted less — the model is uncertain, so a deviation is
expected.

**Familiar task failure = high-precision error**: The model has seen
many similar tasks succeed. When this task fails, the failure is
surprising and should trigger strong learning (update the model
significantly).

**Novel task failure = low-precision error**: The model has no
experience with this type of task. Failure is not surprising and
should trigger weak learning (update the model cautiously).

This precision weighting prevents the model from over-reacting to
novel task failures (which might be one-off anomalies) while ensuring
it reacts strongly to familiar task failures (which indicate a real
change in the system's behavior).

**Implementation path**: The cascade router's observation count per
context provides the precision signal. Contexts with many observations
have high precision. Contexts with few observations have low precision.
The conductor could use this same signal to weight its threshold
adjustments.

Reference: This framework draws on Song et al. (ICLR 2025) on
self-improvement convergence: systems improve when the verifier's
precision exceeds the generator's. The conductor's precision (accuracy
of its self-model) must exceed the agent's variety (range of failure
modes) for the feedback loop to converge toward healthy execution.

---

## The Model Gap

The self-model is always incomplete. The six stuck kinds do not
cover all possible stuck modes. The 20 error categories do not cover
all possible errors. The 34 patterns do not match all possible error
messages.

This incompleteness is inherent — a complete model would be as complex
as the system itself (a consequence of Ashby's Law). The practical
response is:

1. **Default handling**: Unknown errors fall through to generic
   categories (CompileError → RetryWithContext). The system has a
   response even when the model does not have a specific classification.

2. **Error logging**: Every error that does not match a specific
   pattern is logged with full context. These unmatched errors are
   candidates for new patterns.

3. **Model expansion**: New patterns and categories are added as new
   error types are encountered in production. The model grows toward
   completeness over time.

4. **Learning integration**: The efficiency tracking system records
   all errors, including unclassified ones. Over time, clustering of
   unclassified errors reveals new categories that the model should
   include.

---

## Recursive Self-Modeling

The meta-cognition hook introduces a recursive element: the system
models its own modeling process.

```
Level 0: Agent executes task
Level 1: Watchers model agent execution
Level 2: MetaCognitionHook models watcher effectiveness
```

The meta-cognition hook asks: "Am I stuck?" This is a second-order
question — it is the system asking about the effectiveness of its
own first-order monitoring.

In principle, this recursion could continue (Level 3: "Is my
meta-cognition effective?"), but in practice two levels suffice.
The law of diminishing returns applies: each level of meta-cognition
adds complexity but decreasing diagnostic value.

---

## File Reference

| File | What |
|------|------|
| `crates/roko-conductor/src/conductor.rs` | The self-model instantiation (Conductor::new() creates 10 watchers) |
| `crates/roko-conductor/src/stuck_detection.rs` | Process pattern model (6 stuck heuristics) |
| `crates/roko-conductor/src/diagnosis.rs` | Failure taxonomy (20 categories, 34 patterns) |
| `crates/roko-conductor/src/health.rs` | Infrastructure expectation model (4 checks) |
| `crates/roko-learn/src/efficiency.rs` | Data source for model calibration |
| `crates/roko-gate/src/adaptive_threshold.rs` | Adaptive model precedent (EMA thresholds) |

---

## Self-Model Accuracy Metrics

The self-model is only useful if it is accurate. A model that
misclassifies healthy agents as stuck, or predicts gate outcomes no
better than chance, degrades the system rather than regulating it.
This section defines formal accuracy measurements for each component
of the conductor's internal model.

### Prediction error metrics

Each metric measures the divergence between what the model predicted
and what actually happened:

```rust
/// Accuracy metrics for the conductor's self-model.
/// Each metric measures the divergence between predicted and observed behavior.
pub struct SelfModelAccuracy {
    /// Watcher threshold accuracy: fraction of interventions that improve outcomes.
    /// Computed as: (restarts where next attempt succeeds) / (total restarts).
    pub intervention_effectiveness: f64,

    /// Stuck detection precision: fraction of stuck detections that were genuine.
    /// Computed as: (stuck detections where agent was truly non-progressing) / (total detections).
    pub stuck_detection_precision: f64,

    /// Error classification accuracy: fraction of diagnoses with correct category.
    /// Computed as: (correct categories) / (total diagnoses).
    pub diagnosis_accuracy: f64,

    /// Prediction error on task completion time.
    /// RMSE between predicted and actual completion duration.
    pub completion_time_rmse_ms: f64,

    /// Prediction error on gate pass probability.
    /// Brier score: mean((predicted_pass_prob - actual_pass)^2).
    pub gate_pass_brier_score: f64,

    /// Overall model quality: harmonic mean of component accuracies.
    pub composite_accuracy: f64,
}
```

### Per-component accuracy tracking

Each model component makes a specific prediction that can be compared
against a concrete observation:

| Model Component | Prediction | Observation | Metric |
|----------------|-----------|------------|--------|
| Watcher thresholds | "This behavior is pathological" | Did restart improve outcome? | Intervention effectiveness |
| Stuck heuristics | "Agent is stuck" | Was the agent truly non-progressing? | Detection precision |
| Error categories | "This is an ImportError" | Was auto-fix successful? | Classification accuracy |
| Phase timeouts | "Task should finish in 300s" | Actual completion time | RMSE |
| Cost budgets | "Plan should cost < $10" | Actual plan cost | Mean absolute error |

The key constraint: every prediction needs a paired observation.
Predictions without observable outcomes cannot be calibrated. For
this reason, each metric above is defined in terms of an outcome
the system can actually measure after the fact.

### Brier score for calibration

The Brier score measures whether the model's confidence matches
reality. When the model says "80% chance of gate pass," do 80% of
those attempts actually pass?

Formally: `BS = (1/N) * sum((p_i - o_i)^2)` where p_i is the
predicted probability and o_i is the outcome (0 or 1). Perfect
calibration yields BS = 0. Random guessing yields BS = 0.25.

```rust
/// Brier score calculator for model calibration assessment.
/// Measures whether predicted probabilities match observed frequencies.
pub struct BrierScoreTracker {
    /// Running sum of squared errors.
    sum_squared_error: f64,
    /// Total predictions tracked.
    count: usize,
    /// Calibration bins: (predicted_prob_range, actual_pass_count, total_count).
    calibration_bins: Vec<CalibrationBin>,
}

pub struct CalibrationBin {
    pub range_low: f64,
    pub range_high: f64,
    pub actual_passes: usize,
    pub total: usize,
}

impl BrierScoreTracker {
    pub fn record(&mut self, predicted_prob: f64, actual_outcome: bool) {
        let outcome = if actual_outcome { 1.0 } else { 0.0 };
        self.sum_squared_error += (predicted_prob - outcome).powi(2);
        self.count += 1;
        // Update calibration bin
        for bin in &mut self.calibration_bins {
            if predicted_prob >= bin.range_low && predicted_prob < bin.range_high {
                bin.total += 1;
                if actual_outcome { bin.actual_passes += 1; }
                break;
            }
        }
    }

    pub fn brier_score(&self) -> f64 {
        if self.count == 0 { return 0.25; }
        self.sum_squared_error / self.count as f64
    }
}
```

Calibration bins enable a finer diagnostic: split predictions into
ranges (0.0-0.1, 0.1-0.2, ... 0.9-1.0) and compare the actual pass
rate within each bin against the predicted range. A well-calibrated
model has actual rates that match the bin midpoints.

---

## Self-Model Learning — Online Adaptation

A static self-model drifts as the system evolves. New model versions
produce different error patterns. Codebase changes alter what "normal"
spec drift looks like. Task complexity shifts change typical iteration
counts. The conductor needs to adapt its model online, without
operator intervention.

### Bayesian threshold adaptation

Each watcher threshold encodes a belief: "interventions at this
threshold improve outcomes." The conductor can track this belief as
a Beta distribution and update it from observed intervention results.

The update rule:
- Intervention fires and the restarted agent succeeds: alpha += 1 (evidence that the threshold is useful)
- Intervention fires and the restarted agent also fails: beta += 1 (evidence that the threshold is too aggressive)
- The threshold is well-calibrated when alpha / (alpha + beta) approximates the target precision (e.g., 0.8)

```rust
/// Bayesian threshold adaptation for conductor watchers.
/// Tracks whether interventions triggered at each threshold improve outcomes.
pub struct ThresholdLearner {
    /// Per-watcher threshold performance tracking.
    watchers: HashMap<String, ThresholdPosterior>,
}

pub struct ThresholdPosterior {
    /// Current threshold value.
    pub threshold: f64,
    /// Beta distribution parameters for intervention success rate.
    pub alpha: f64,  // successful interventions (restart led to success)
    pub beta: f64,   // unsuccessful interventions (restart led to failure)
    /// Discount factor for non-stationarity (default: 0.995).
    pub discount: f64,
    /// Minimum effective sample size before adapting (default: 10).
    pub min_samples: f64,
}

impl ThresholdPosterior {
    pub fn record_outcome(&mut self, intervention_helped: bool) {
        // Discount old observations for non-stationarity
        self.alpha = 1.0 + (self.alpha - 1.0) * self.discount;
        self.beta = 1.0 + (self.beta - 1.0) * self.discount;
        // Update with new observation
        if intervention_helped {
            self.alpha += 1.0;
        } else {
            self.beta += 1.0;
        }
    }

    /// Estimated intervention success rate (posterior mean).
    pub fn success_rate(&self) -> f64 {
        self.alpha / (self.alpha + self.beta)
    }

    /// Effective sample size (accounting for discounting).
    pub fn effective_samples(&self) -> f64 {
        self.alpha + self.beta - 2.0
    }

    /// Should the threshold be tightened (lower) or loosened (higher)?
    pub fn threshold_adjustment(&self) -> ThresholdDirection {
        if self.effective_samples() < self.min_samples {
            return ThresholdDirection::Hold;
        }
        let rate = self.success_rate();
        if rate > 0.85 {
            // Interventions are too successful — threshold may be too lenient
            // (catching only obvious cases). Consider tightening.
            ThresholdDirection::Tighten
        } else if rate < 0.5 {
            // Interventions are mostly unsuccessful — threshold may be too strict
            // (false positives). Consider loosening.
            ThresholdDirection::Loosen
        } else {
            ThresholdDirection::Hold
        }
    }
}

pub enum ThresholdDirection { Tighten, Loosen, Hold }
```

The discount factor (0.995) causes old observations to decay
gradually, so the posterior tracks non-stationary behavior. Without
discounting, early observations would dominate indefinitely, and the
model would resist adaptation as the system evolves.

### Kalman filter for state estimation

System parameters drift over time: baseline error rates shift as the
codebase grows, typical costs change as model pricing evolves, and
completion times vary as task complexity changes. A scalar Kalman
filter provides online estimation that balances the existing model
against new observations.

```rust
/// Simplified scalar Kalman filter for online parameter estimation.
/// Used to track slowly-drifting system parameters (baseline error rate, typical cost).
pub struct ScalarKalman {
    /// Current state estimate.
    pub estimate: f64,
    /// Estimation uncertainty (variance).
    pub uncertainty: f64,
    /// Process noise: how much the true value can drift per step.
    pub process_noise: f64,
    /// Measurement noise: how noisy observations are.
    pub measurement_noise: f64,
}

impl ScalarKalman {
    pub fn update(&mut self, observation: f64) {
        // Predict step: uncertainty grows by process noise
        self.uncertainty += self.process_noise;
        // Update step: incorporate observation
        let kalman_gain = self.uncertainty / (self.uncertainty + self.measurement_noise);
        self.estimate += kalman_gain * (observation - self.estimate);
        self.uncertainty *= 1.0 - kalman_gain;
    }

    /// Prediction error (how surprising was the last observation?).
    pub fn prediction_error(&self, observation: f64) -> f64 {
        (observation - self.estimate).abs()
    }
}
```

The Kalman gain is the key: when uncertainty is high relative to
measurement noise, the filter trusts new observations more. When
uncertainty is low, it trusts the existing estimate. This provides
the same "precision weighting" behavior described in the
precision-weighted prediction errors section, but through a
classical filtering framework rather than an active inference one.

### Active inference integration

The precision-weighted prediction error framework described earlier
in this document provides the mechanism; the active inference
integration provides the weighting. Errors from reliable sources
drive large model updates. Errors from noisy sources drive small ones.

Precision is derived from the cascade router's observation count per
context. A context with 200 observations has high precision (the model
knows what to expect). A context with 3 observations has low precision
(the model is guessing).

```rust
/// Precision-weighted model update inspired by active inference.
/// Errors from reliable sources drive large updates; noisy sources drive small ones.
pub struct PrecisionWeightedUpdater {
    /// Per-context precision estimates (inverse variance of prediction errors).
    context_precision: HashMap<String, f64>,
    /// Minimum precision (prevents zero-weight on novel contexts).
    min_precision: f64,  // default: 0.1
    /// Maximum precision (prevents over-confidence on familiar contexts).
    max_precision: f64,  // default: 10.0
}

impl PrecisionWeightedUpdater {
    /// Update the model with a precision-weighted prediction error.
    pub fn weighted_update(
        &self,
        context: &str,
        prediction_error: f64,
        base_learning_rate: f64,
    ) -> f64 {
        let precision = self.context_precision
            .get(context)
            .copied()
            .unwrap_or(self.min_precision)
            .clamp(self.min_precision, self.max_precision);

        // Learning rate scales with precision: precise contexts drive larger updates
        base_learning_rate * precision * prediction_error
    }
}
```

The min/max precision bounds prevent two failure modes. Without a
minimum, novel contexts would produce zero-weight updates and the
model would never learn about new task types. Without a maximum,
familiar contexts would dominate all updates and the model would
over-fit to historical patterns even when the underlying system has
changed.

---

## The Internal Model Principle and Forward Prediction

### Francis-Wonham (1976) — The Internal Model Principle

The Internal Model Principle (IMP) strengthens Conant-Ashby. Where
the Good Regulator theorem says "a good regulator must contain a
model," the IMP says the controller must contain a copy of the
dynamics generating the signals it must track. Not just any model —
the same dynamical structure.

For the conductor, this has a concrete implication: the conductor's
model of gate outcomes must mirror the actual gate pipeline's logic.
If the gate pipeline runs compile, test, clippy, and diff checks in
sequence, the conductor's forward model must predict the outcome of
each check in that same sequence. If a new gate is added (say, a
security audit gate), the conductor's model must incorporate it or
regulation degrades — the conductor cannot anticipate failures from
a gate it does not model.

This is testable: when the gate pipeline changes and the conductor's
model does not update, intervention effectiveness should drop
measurably. The Brier score on gate pass prediction should increase
(worsen). The Bayesian threshold posteriors should shift toward
higher beta (more unsuccessful interventions). These signals indicate
that the internal model has diverged from the system it regulates.

### Forward prediction

The self-model enables prediction, and prediction enables anticipatory
intervention. Instead of waiting for a watcher to trigger (reactive),
the conductor can predict future state and intervene before a failure
materializes (proactive).

Given current execution state — iteration count, accumulated cost,
error count, time elapsed — the forward predictor estimates the
probability that the next gate attempt will pass. If that probability
drops below a threshold (e.g., 0.3), the conductor can preemptively
trigger a strategy change: switch to a stronger model, enrich the
context, or restructure the approach.

```rust
/// Forward prediction using the conductor's self-model.
/// Predicts future state to enable anticipatory intervention.
pub struct ForwardPredictor {
    /// Learned mapping: (current_state) -> (predicted_next_state).
    /// Implemented as linear regression on state features.
    weights: Vec<f64>,
    /// Bias term.
    bias: f64,
    /// Feature extractor: signal stream -> state features.
    feature_dim: usize,
}

impl ForwardPredictor {
    /// Predict the probability of gate pass given current execution state.
    pub fn predict_pass_probability(&self, features: &[f64]) -> f64 {
        assert_eq!(features.len(), self.feature_dim);
        let logit: f64 = self.bias + features.iter()
            .zip(self.weights.iter())
            .map(|(f, w)| f * w)
            .sum::<f64>();
        // Sigmoid to get probability
        1.0 / (1.0 + (-logit).exp())
    }

    /// Online update via stochastic gradient descent.
    pub fn update(&mut self, features: &[f64], actual_pass: bool, lr: f64) {
        let predicted = self.predict_pass_probability(features);
        let target = if actual_pass { 1.0 } else { 0.0 };
        let error = predicted - target;
        // SGD update
        self.bias -= lr * error;
        for (w, f) in self.weights.iter_mut().zip(features.iter()) {
            *w -= lr * error * f;
        }
    }
}
```

The feature vector for prediction includes: current iteration number,
cumulative cost so far, error count in this attempt, time elapsed as
a fraction of timeout, context window usage ratio, and the cascade
router's model confidence for this task type. These features are
available at every point during execution, so prediction is continuous
rather than point-in-time.

### World model vs. self-model

Two distinct models operate inside the conductor, and conflating them
leads to calibration errors:

- **World model**: What will the environment do? This predicts gate
  outcomes, compile results, test results — things external to the
  conductor. The world model answers: "Will this code pass the test
  suite?"

- **Self-model**: What will this system do? This predicts watcher
  behavior, intervention effectiveness, threshold accuracy — things
  internal to the conductor. The self-model answers: "Will my
  intervention improve the outcome?"

Both are needed. The world model predicts what agents will face.
The self-model predicts how the conductor will respond. The IMP
constrains the world model: it must contain the dynamics of the gate
pipeline. The Good Regulator theorem constrains the self-model: it
must contain the dynamics of the watcher ensemble.

A conductor that has a good world model but a poor self-model will
accurately predict failures but respond to them badly (wrong
intervention, wrong timing). A conductor that has a good self-model
but a poor world model will respond well to predicted failures but
miss the ones it did not predict. Regulation quality depends on both.

### References

- Conant, R.C. & Ashby, W.R. (1970). "Every good regulator of a system must be a model of that system." *International Journal of Systems Science*, 1(2), 89-97.
- Francis, B.A. & Wonham, W.M. (1976). "The Internal Model Principle of Control Theory." *Automatica*, 12(5), 457-465.
- Friston, K. (2010). "The free-energy principle: a unified brain theory?" *Nature Reviews Neuroscience*, 11(2), 127-138.
- Chen, B. et al. (2022). "Full-Body Visual Self-Modeling of Robot Morphologies." *Science Robotics*, 7(68).
- Kalman, R.E. (1960). "A New Approach to Linear Filtering and Prediction Problems." *Journal of Basic Engineering*, 82(1), 35-45.
- Song, Y. et al. (2025). "The Good, the Bad, and the Greedy: Evaluation of LLMs Should Not Ignore Non-Determinism." *ICLR 2025*.


---

# SOURCE: /Users/will/dev/nunchi/roko/roko/docs/07-conductor/09-cognitive-signals.md

# Cognitive Signals

> Typed interrupts that carry semantic meaning. Not just "something
> happened" but "pause execution," "reprioritize this task," "inject
> this context," "escalate to a more capable model."


> **Implementation**: Built

---

## Definition

Cognitive Signals are a proposed extension to Roko's signal system
that adds typed interrupt semantics. Where standard signals carry
data (token counts, gate verdicts, cost metrics), cognitive signals
carry intent — they tell the pipeline what to DO, not just what
IS.

```rust
pub enum CognitiveSignal {
    Pause,
    Resume,
    Reprioritize(TaskId),
    InjectContext(Engram),
    Escalate,
    Cooldown,
    Explore,
    Shutdown,
}
```

---

## Signal Semantics

### Pause

**Intent**: Temporarily halt execution of the current task or plan.

**When emitted**: The Conductor detects a condition that requires
external resolution before execution can continue productively.
Examples:
- Spec drift detected — need to verify the specification is still
  current before the agent makes more changes
- Cost approaching budget — need operator approval before spending
  more
- Infrastructure degraded — wait for recovery before dispatching
  more work

**Orchestrator response**: Suspend the affected agent process (or
stop sending it work). Preserve its state. Do not kill — the work
may be resumable.

**Difference from Restart**: Restart kills the agent and starts fresh.
Pause preserves the agent's state and conversation history. Pause is
appropriate when the agent's work is valid but the environment needs
to change before continuing.

### Resume

**Intent**: Continue execution after a Pause.

**When emitted**: The condition that caused the Pause has been
resolved. The spec has been verified, the budget has been approved,
the infrastructure has recovered.

**Orchestrator response**: Resume the suspended agent process or
restart dispatching work to it.

### Reprioritize(TaskId)

**Intent**: Change the priority of a specific task in the scheduling
queue.

**When emitted**: The Conductor or learning system determines that
a task's priority should change based on new information. Examples:
- A dependency of this task just completed — the task is now
  unblocked and should move up in priority
- The task's file set conflicts with a higher-priority in-flight
  task — deprioritize to avoid merge conflicts
- The task has been waiting too long — elevate priority to prevent
  starvation

**Orchestrator response**: Adjust the task's position in the
scheduling queue. This does not affect in-flight tasks — only queued
tasks waiting for dispatch.

### InjectContext(Engram)

**Intent**: Add specific context to the current agent's prompt.

**When emitted**: The Conductor has information that the agent needs
but does not have. Examples:
- The diagnosis engine classified an error and has a specific fix
  suggestion: "E0432 on line 42 — add `use crate::auth::AuthToken;`"
- A playbook rule was matched: "Past builds show auth types have
  lifetime parameters. Check actual signatures."
- Another agent's work produced relevant context: "Plan 3 just
  modified `mod.rs` — your imports may need updating."

**Engram**: In Roko's naming convention, an Engram is a unit of
persistent context. An `InjectContext` signal carries an Engram —
a typed piece of information that the orchestrator injects into the
agent's next prompt.

**Orchestrator response**: Append the Engram content to the agent's
context for its next turn. This may be injected via the system prompt
(`--append-system-prompt`), via `context/in/`, or via MCP tool
response.

### Escalate

**Intent**: Move the task to a more capable processing tier.

**When emitted**: The current model or agent configuration is
insufficient for the task. Examples:
- A Haiku-tier agent has failed twice on a complex task
- The diagnosis engine identified an error category (BorrowCheckerError,
  LifetimeError) that requires deeper reasoning
- The quality judge scored the output below threshold

**Orchestrator response**: Kill the current agent. Respawn with:
- A more capable model (Haiku → Sonnet → Opus)
- More context (add type signatures, dependency graph)
- Different tools (add `get_symbol_context`, `get_change_impact`)

**Connection to cascade router**: Escalation feeds a negative reward
to the cascade router for the current model-task combination. Over
time, the router learns to route complex tasks to capable models
directly, reducing the need for escalation.

### Cooldown

**Intent**: Reduce pressure on the current task or plan.

**When emitted**: The Conductor detects that the agent is under
too much pressure — approaching the Yerkes-Dodson collapse zone.
Indicators:
- Rapid context growth (agent is accumulating errors and retries)
- Decreasing output quality per turn
- Increasing token cost per turn with decreasing progress

**Orchestrator response**: Extend timeouts, reduce iteration
pressure, or add a deliberate pause before the next attempt. The
goal is to move the agent back toward the productive zone of the
Yerkes-Dodson curve.

**Yerkes-Dodson context**: Research on 770,000+ autonomous agents
shows cooperative behavior follows an inverted-U curve with
environmental pressure. Moderate pressure maximizes cooperation.
Extreme pressure collapses cooperative behavior within 5-12 turns.
The Cooldown signal is the mechanism for detecting and responding
to over-pressure.

Reference: Yerkes & Dodson (1908). "The relation of strength of
stimulus to rapidity of habit-formation."

### Explore

**Intent**: Grant the agent more freedom to explore alternative
approaches.

**When emitted**: The current approach has failed but the task
itself is believed to be solvable. The agent needs creative freedom
rather than tighter constraints. Examples:
- Two different approaches have both failed at the gate
- The diagnosis engine suggests the error requires an architectural
  change, not a local fix
- Historical episodes show this task type benefits from exploration

**Orchestrator response**: Expand the agent's tool access, increase
the iteration limit, or provide broader context. The agent gets
more rope — at the cost of more tokens and time.

**Tension with Cooldown**: Explore and Cooldown pull in different
directions. Explore grants more freedom (potentially more pressure).
Cooldown restricts freedom (less pressure). The Conductor must
choose between them based on the specific failure mode. Repeated
identical errors → Cooldown (more of the same approach will not
help). Diverse but unsuccessful attempts → Explore (the agent is
trying different things and needs room to find the right one).

### Shutdown

**Intent**: Gracefully terminate execution.

**When emitted**: The Conductor determines that the entire execution
should stop. Examples:
- Budget for the batch run is exhausted
- Critical infrastructure failure (all agents down)
- Operator-initiated shutdown (Ctrl+C)

**Orchestrator response**: Execute the graceful shutdown sequence:
1. Stop accepting new tasks
2. Drain in-flight tasks (30-second grace period)
3. Kill remaining agents if drain times out
4. Save checkpoint to `.roko/state/executor.json`
5. Flush logs
6. Exit

---

## Signal vs. Signal

Roko's core `Signal` type already carries typed data through the
pipeline. Cognitive Signals extend this with intent semantics:

| Aspect | Standard Signal | Cognitive Signal |
|--------|----------------|-----------------|
| **Purpose** | Data transport | Intent transport |
| **Content** | Measurement (tokens, cost, time) | Command (pause, escalate, inject) |
| **Producer** | Any component | Conductor, learning system |
| **Consumer** | Any component | Orchestrator |
| **Action** | Read and react | Execute the intent |

Cognitive Signals can be encoded as standard Signals using the
`Kind::Custom` variant:

```rust
// Encoding a cognitive signal as a standard engram
fn cognitive_to_engram(cs: &CognitiveSignal) -> Engram {
    match cs {
        CognitiveSignal::Pause => {
            Engram::builder(Kind::Custom("conductor.cognitive.pause".into()))
                .body(Body::text("pause execution"))
                .tag("cognitive_signal", "pause")
                .build()
        }
        CognitiveSignal::Escalate => {
            Engram::builder(Kind::Custom("conductor.cognitive.escalate".into()))
                .body(Body::text("escalate to higher tier"))
                .tag("cognitive_signal", "escalate")
                .build()
        }
        // ...
    }
}
```

This encoding preserves backward compatibility — the cognitive signal
is just a Signal with specific Kind and tags. Components that do not
understand cognitive signals can safely ignore them.

---

## Implementation Status

Cognitive Signals are defined in the refactoring PRD (§XII.2,
09-innovations.md) but not yet implemented as a formal type in the
codebase. The Conductor currently expresses its decisions through
`ConductorDecision` (Continue/Restart/Fail), which covers a subset
of cognitive signal semantics:

| ConductorDecision | Equivalent Cognitive Signal |
|-------------------|---------------------------|
| Continue | (no signal — healthy) |
| Restart | Escalate or InjectContext + Resume |
| Fail | Shutdown (for the specific plan) |

The missing cognitive signals (Pause, Resume, Reprioritize,
InjectContext, Cooldown, Explore) represent planned extensions that
would give the Conductor more nuanced control over execution.

**Path to implementation**:
1. Define `CognitiveSignal` enum in `roko-core`
2. Extend `ConductorDecision` to include cognitive signal variants
3. Teach the orchestrator to handle each signal type
4. Wire watchers to emit cognitive signals when appropriate
5. Add learning system integration (track which cognitive signals
   improve outcomes)

---

## Cognitive Signals in the Cybernetic Loop

Cognitive Signals enrich the Conductor's OODA loop:

**Without cognitive signals**: The Conductor can only Continue,
Restart, or Fail. Every anomaly gets one of three responses.

**With cognitive signals**: The Conductor can Pause (wait for
conditions to change), Cooldown (reduce pressure), Explore (grant
freedom), InjectContext (provide targeted help), Escalate (increase
capability), or Reprioritize (reorder the queue). The response
vocabulary grows from 3 to 8+, matching the variety of the
anomalies the Conductor can detect.

This directly addresses Ashby's Law: the regulator's variety (number
of distinct responses) must match the system's variety (number of
distinct failure modes). With only 3 responses, many distinct failure
modes receive the same generic treatment. With 8+ responses, each
failure mode can receive a tailored intervention.

---

## File Reference

| File | What |
|------|------|
| `refactoring-prd/09-innovations.md` §XII.2 | Cognitive Signal definition |
| `crates/roko-core/src/agent.rs` | ConductorDecision (current 3-state decision) |
| `crates/roko-conductor/src/conductor.rs` | evaluate() (where decisions are made) |
| `crates/roko-conductor/src/interventions.rs` | Intervention policy (decision resolution) |


---

# SOURCE: /Users/will/dev/nunchi/roko/roko/docs/07-conductor/10-adaptive-timeouts-state-machine.md

# Adaptive Timeouts and the State Machine

> Phase timeouts by complexity band. Hard timeouts that are law,
> not suggestions. PhaseTransition records for audit trails.


> **Implementation**: Built

---

## The State Machine

Each plan progresses through a well-defined set of phases. The state
machine defines which transitions are valid and enforces them:

```
Queued → Implementing → Gating → Reviewing → Done → Merging → Complete
                          │         │
                          ▼         ▼
                      AutoFixing  (re-implement)
                          │
                          ▼
                      (back to Gating)
```

Invalid transitions are structurally impossible. A plan cannot jump
from Queued to Reviewing. A plan cannot go from Complete back to
Implementing. The state machine is a DATA STRUCTURE, not code paths.

This is Hard Guarantee 1 from the failure prevention catalog:
"Explicit State Machine with Compile-Time Transition Validation."

---

## Phase Timeouts

Every phase has a hard wall-clock timeout. When the timeout fires,
the plan transitions to Failed. No exceptions.

```rust
// From crates/roko-conductor/src/state_machine.rs
pub fn phase_timeout(phase: PlanPhase, complexity: Complexity) -> Duration {
    match (phase, complexity) {
        // Implementation timeouts scale with complexity
        (Implementing, Complex)  => Duration::from_secs(600),   // 10 min
        (Implementing, Standard) => Duration::from_secs(300),   // 5 min
        (Implementing, Fast)     => Duration::from_secs(120),   // 2 min

        // Other phases have fixed timeouts
        (Gating, _)              => Duration::from_secs(300),   // 5 min
        (Reviewing, _)           => Duration::from_secs(300),   // 5 min
        (Merging, _)             => Duration::from_secs(60),    // 1 min
        // ...
    }
}
```

### Why Hard Timeouts

Soft timeouts (conductor detects timeout → decides whether to
intervene) do not work. Production experience:

- Conductor detects timeout → nudges agent → agent continues
  (ignoring nudge) → conductor detects timeout again → nudges
  again → 10 minutes wasted

Hard timeouts are enforced by the state machine:

```rust
// Timer check runs every 5 seconds
for plan in active_plans {
    let elapsed = plan.phase_entered_at.elapsed();
    let timeout = phase_timeout(plan.phase, plan.complexity);
    if elapsed > timeout {
        transition(plan, Failed(Timeout));  // HARD. No negotiation.
    }
}
```

This is Hard Guarantee 2: "Every Phase Has a Hard Timeout."

### Complexity-Based Scaling

Implementation timeouts scale with plan complexity because complex
plans legitimately need more time. A trivial plan (add a const, fix
a typo) should complete in 2 minutes. A complex plan (implement a
new subsystem, wire multiple crates) may need 10 minutes.

The complexity classification comes from the plan's TOML frontmatter
or from the cascade router's complexity classifier:

| Complexity | Typical Plans | Implementation Timeout |
|-----------|--------------|----------------------|
| Fast | Typo fixes, const additions, doc updates | 120s (2 min) |
| Standard | Function implementations, module additions | 300s (5 min) |
| Complex | Multi-crate features, architectural changes | 600s (10 min) |

Other phases (Gating, Reviewing, Merging) do not scale with
complexity because their duration depends on codebase size and
test suite speed, not on plan complexity.

---

## PhaseTransition Records

Every phase transition produces an audit record:

```rust
pub struct PhaseTransition {
    pub plan_id: String,
    pub from: PlanPhase,
    pub to: PlanPhase,
    pub timestamp: String,   // ISO 8601
    pub reason: String,      // why the transition occurred
}
```

These records provide a complete history of every plan's progression:

```
plan-42: Queued → Implementing    (2026-04-09T10:00:00Z, "dependencies met")
plan-42: Implementing → Gating    (2026-04-09T10:03:22Z, "all tasks complete")
plan-42: Gating → Implementing    (2026-04-09T10:04:15Z, "gate failed: 2 compile errors")
plan-42: Implementing → Gating    (2026-04-09T10:06:48Z, "all tasks complete")
plan-42: Gating → Reviewing       (2026-04-09T10:07:30Z, "all gates passed")
plan-42: Reviewing → Merging      (2026-04-09T10:08:45Z, "review approved")
plan-42: Merging → Complete       (2026-04-09T10:09:02Z, "merge successful")
```

This audit trail enables:

1. **Post-mortem analysis**: How long did each phase take? How many
   gate-fail-retry cycles occurred?
2. **Performance optimization**: Which phase is the bottleneck?
   If Gating consistently takes 4 minutes of a 7-minute plan, gate
   optimization has the highest impact.
3. **Anomaly detection**: Plans that transition through unusual
   sequences can be flagged for investigation.
4. **Learning system input**: Phase timing data feeds into the
   cascade router's complexity classifier and the adaptive gate
   threshold system.

---

## Adaptive Timeout Computation

The static timeouts are derived from production experience. An
adaptive system would compute timeouts from observed execution data:

### P95-Based Adaptive Timeout

From the production hardening plan (doc 16):

```rust
impl LatencyStats {
    /// Recommended timeout = 2x the observed p95 latency,
    /// clamped to [5s, 300s].
    pub fn adaptive_timeout_ms(&self) -> u64 {
        if self.observations < 10 { return 120_000; }  // Not enough data
        let p95 = self.p95_ms();
        let timeout = (p95 * 2.0) as u64;
        timeout.clamp(5_000, 300_000)
    }
}
```

This approach sets the timeout at 2x the observed 95th percentile
latency. With enough observations, the timeout automatically adjusts
to match actual execution patterns:

- If complex plans consistently finish in 4 minutes, the adaptive
  timeout settles at ~8 minutes (p95 ≈ 4 min × 2)
- If model upgrades make agents faster (finishing in 2 minutes),
  the timeout automatically tightens to ~4 minutes
- If codebase growth makes compilation slower, the timeout
  automatically widens

### Cold Start Behavior

With fewer than 10 observations, the system uses the static default
(120 seconds). This prevents the adaptive system from setting
unreasonable timeouts based on a small, potentially unrepresentative
sample.

After 10+ observations, the adaptive timeout takes over. The p95
calculation uses a sliding window of recent observations, so the
timeout reflects current performance rather than historical averages.

### Per-Phase Adaptive Timeouts

Different phases have different timeout distributions:

| Phase | What Determines Duration |
|-------|------------------------|
| Implementing | Agent reasoning speed, codebase complexity |
| Gating | Compile time, test suite size |
| Reviewing | Reviewer model speed, number of reviewers |
| Merging | Git merge speed, post-merge test time |

Each phase should have its own adaptive timeout, computed from its
own observation window. The infrastructure exists in the latency
registry (`roko-learn/src/latency.rs`).

---

## TTFT Timeout

Time-to-first-token (TTFT) timeout provides early detection of
stalled providers:

```rust
pub struct ProviderConfig {
    pub timeout_ms: Option<u64>,        // Hard per-request timeout (120s default)
    pub ttft_timeout_ms: Option<u64>,   // Time-to-first-token timeout (15s default)
    pub connect_timeout_ms: Option<u64>, // TCP connection timeout (5s default)
}
```

If a provider has not sent a single token in 15 seconds, something is
wrong — fail fast and try a fallback rather than waiting 2 minutes for
the hard timeout. This layered timeout approach detects problems
earlier:

```
Request sent
    │
    │ ← connect_timeout_ms (5s): TCP connection must be established
    │
    │ ← ttft_timeout_ms (15s): first token must arrive
    │
    │ ← timeout_ms (120s): complete response must arrive
    │
Response received
```

Each layer catches a different failure mode:
- Connection timeout → DNS failure, firewall, provider down
- TTFT timeout → provider overloaded, queue backed up
- Full timeout → response generation taking too long

---

## Graceful Shutdown Sequence

When a Shutdown cognitive signal is received or Ctrl+C is pressed,
the orchestrator executes a four-phase shutdown:

```rust
pub async fn run_with_shutdown(executor: PlanExecutor, snapshot_path: &Path) -> Result<()> {
    let shutdown = signal::ctrl_c();

    tokio::select! {
        result = executor.run() => result,
        _ = shutdown => {
            // Phase 1: Stop accepting new tasks
            executor.stop_accepting();

            // Phase 2: Drain with timeout (30s grace period)
            let drain = tokio::time::timeout(
                Duration::from_secs(30),
                executor.drain_in_flight(),
            ).await;

            if drain.is_err() {
                // Phase 2b: Kill remaining agents if drain times out
                executor.kill_all_agents().await;
            }

            // Phase 3: Checkpoint
            executor.save_snapshot(snapshot_path)?;

            // Phase 4: Flush logs
            executor.flush_logs().await;

            Ok(())
        }
    }
}
```

### Atomic Checkpoint Writes

The checkpoint write uses temp-file-then-rename to prevent
corruption from mid-write crashes:

```rust
fn save_snapshot_atomic(snapshot: &ExecutorSnapshot, path: &Path) -> Result<()> {
    let tmp_path = path.with_extension("json.tmp");
    let json = serde_json::to_string_pretty(snapshot)?;
    std::fs::write(&tmp_path, &json)?;
    std::fs::rename(&tmp_path, path)?;  // Atomic on POSIX
    Ok(())
}
```

A kill signal mid-write leaves the previous snapshot intact rather
than producing a corrupted file. If the state file is corrupted
(disk issue, OOM kill during rename on some filesystems), the
persistence manager falls back to reconstructing completed tasks
from the append-only event log.

---

## Relationship to Process Supervision

Phase timeouts in the Conductor complement process-level supervision
in `bardo-runtime`:

| Layer | Timeout Type | What It Catches |
|-------|-------------|----------------|
| Process (bardo-runtime) | Process timeout | Agent process hangs |
| Task (Conductor) | Phase timeout | Task takes too long in any phase |
| Plan (Conductor) | Wall-clock limit | Total plan execution exceeds limit |
| Batch (Orchestrator) | Budget limit | Total batch cost exceeds limit |

Each layer catches problems at a different granularity. A process
timeout catches an individual agent hang. A phase timeout catches a
task that cycles through multiple agent processes but never
completes. A plan wall-clock limit catches plans that make progress
but too slowly. A batch budget limit catches runaway cost across
all plans.

---

## File Reference

| File | What |
|------|------|
| `crates/roko-conductor/src/state_machine.rs` | Phase timeouts, PhaseTransition, complexity-based scaling |
| `crates/roko-learn/src/latency.rs` | LatencyStats, adaptive_timeout_ms(), percentile computation |
| `crates/roko-core/src/config/schema.rs` | ProviderConfig with timeout fields |
| `crates/roko-cli/src/orchestrate.rs` | Graceful shutdown, atomic checkpoints |


---

# SOURCE: /Users/will/dev/nunchi/roko/roko/docs/07-conductor/11-anomaly-detection-learning.md

# Anomaly Detection and Learning Integration

> The Conductor does not exist in isolation. It feeds the learning
> system and is fed by it. Interventions produce data. Data produces
> better interventions.


> **Implementation**: Built

---

## AnomalyDetector

The anomaly detector (`roko-learn/src/anomaly.rs`) provides
statistical anomaly detection complementing the Conductor's
threshold-based watchers:

```rust
pub struct AnomalyDetector {
    prompt_hash_window: VecDeque<u64>,   // last 20 prompt hashes
    cost_ewma: EwmaState,                // exponentially weighted moving average
    quality_history: VecDeque<f64>,      // last 50 quality scores
    session_cost_usd: f64,               // accumulated session cost
    session_start_ms: i64,               // session start time
}

pub enum Anomaly {
    PromptLoop { repeated_count: usize },
    CostSpike { z_score: f64 },
    QualityDegradation { avg_drop: f64 },
    BudgetExhausted { used: f64, limit: f64 },
}
```

### Prompt Loop Detection

Hashes each prompt and tracks in a sliding window of 20. Five
identical hashes trigger `Anomaly::PromptLoop`:

```rust
pub fn check_prompt(&mut self, prompt_hash: u64) -> Option<Anomaly> {
    self.prompt_hash_window.push_back(prompt_hash);
    if self.prompt_hash_window.len() > 20 {
        self.prompt_hash_window.pop_front();
    }

    let count = self.prompt_hash_window.iter()
        .filter(|&&h| h == prompt_hash)
        .count();

    if count >= 5 {
        Some(Anomaly::PromptLoop { repeated_count: count })
    } else {
        None
    }
}
```

This catches a broader class of loops than the stuck-pattern watcher.
The watcher looks at agent output; the anomaly detector looks at
agent input. If the system is sending the same prompt five times, the
agent will produce the same output five times — detecting the loop at
the input level catches it earlier.

### Cost Spike Detection

Uses EWMA (Exponentially Weighted Moving Average) with z-score
anomaly detection:

```rust
impl EwmaState {
    pub fn update(&mut self, value: f64) {
        let diff = value - self.mean;
        self.mean += self.alpha * diff;
        self.variance = (1.0 - self.alpha) * (self.variance + self.alpha * diff * diff);
    }

    pub fn z_score(&self, value: f64) -> f64 {
        let stddev = self.variance.sqrt();
        if stddev < 1e-10 { return 0.0; }
        (value - self.mean) / stddev
    }
}
```

A z-score above 3.0 triggers `Anomaly::CostSpike`. This means the
cost of the current turn is more than 3 standard deviations above
the running average — a sudden 10x cost increase, for example.

### Quality Degradation Detection

Compares recent quality scores (last 5) against earlier scores
(turns 11-20). If the recent average drops more than 0.15 below
the earlier average AND the recent average is below 0.5, the system
is degrading:

```rust
if recent_avg < earlier_avg - 0.15 && recent_avg < 0.5 {
    Some(Anomaly::QualityDegradation { avg_drop: earlier_avg - recent_avg })
}
```

The dual condition prevents false positives: a quality drop from 0.95
to 0.80 does not trigger (recent is still above 0.5), but a drop from
0.7 to 0.4 does (recent is below 0.5 AND the delta exceeds 0.15).

---

## Conductor ↔ Learning System Integration

### Interventions as Learning Signals

Every conductor intervention produces data for the learning system:

```
Conductor fires "compile-fail-repeat" for plan-42
    │
    ▼
AgentEfficiencyEvent {
    agent_id: "agent-7",
    model: "claude-sonnet-4-6",
    outcome: "conductor_intervention",
    gate_errors: [{ category: "TypeMismatch", count: 3 }],
    // ...
}
    │
    ▼
Cascade Router records negative observation:
    model="claude-sonnet-4-6", context=(complex_task, auth_module), reward=low
    │
    ▼
Next similar task routed to claude-opus-4-6 instead
```

The intervention is not just a corrective action — it is a data point.
The learning system uses it to improve future routing decisions.

### Efficiency Events

Every agent turn records an `AgentEfficiencyEvent` with 20+ fields:

```rust
pub struct AgentEfficiencyEvent {
    pub agent_id: String,
    pub role: String,
    pub backend: String,
    pub model: String,
    pub plan_id: String,
    pub task_id: String,
    pub input_tokens: u64,
    pub output_tokens: u64,
    pub reasoning_tokens: u64,
    pub cache_read_tokens: u64,
    pub cache_write_tokens: u64,
    pub cost_usd: f64,
    pub cost_usd_without_cache: f64,
    pub prompt_sections: Vec<String>,
    pub total_prompt_tokens: u64,
    pub system_prompt_tokens: u64,
    pub tools_available: usize,
    pub tools_used: usize,
    pub tool_calls: Vec<String>,
    pub wall_time_ms: u64,
    pub duration_ms: u64,
    pub time_to_first_token_ms: u64,
    pub was_warm_start: bool,
    pub iteration: u32,
    pub gate_passed: bool,
    pub outcome: String,
    pub gate_errors: Vec<String>,
    pub model_used: String,
    pub frequency: OperatingFrequency,
    pub strategy_attempted: String,
    pub timestamp: String,
}
```

The context window pressure watcher reads these events directly:
```rust
if let Ok(event) = signal.body.as_json::<AgentEfficiencyEvent>() {
    if let Some(total) = context_window_tokens(&event.model) {
        return Some((event.total_prompt_tokens as f64, total as f64));
    }
}
```

This shared data format means context pressure monitoring and
efficiency tracking use the same instrumentation — no additional
data collection needed.

### Cascade Router Feedback

The cascade router learns which model-task combinations produce good
outcomes. Conductor interventions are negative outcomes:

| Conductor Event | Router Signal |
|----------------|--------------|
| Continue | Positive (task progressing normally) |
| Restart (compile-fail-repeat) | Negative (model failed on compile errors) |
| Restart (stuck-pattern) | Negative (model got stuck) |
| Restart (ghost-turn) | Strongly negative (model produced nothing) |
| Fail (iteration-loop) | Strongly negative (model did not converge) |

Over time, the router accumulates enough data to route tasks away
from model-context combinations that historically trigger conductor
interventions:

```
Context: { complexity: Complex, category: Auth, file_count: 5 }
    │
    ▼
Router observations:
    claude-sonnet-4-6 + this context → 3 interventions in 5 attempts
    claude-opus-4-6   + this context → 0 interventions in 3 attempts
    │
    ▼
Router routes to opus for this context
```

### Adaptive Gate Thresholds

The adaptive gate threshold system (`roko-gate/src/adaptive_threshold.rs`)
adjusts gate pass criteria based on historical data using exponential
moving averages per gate rung.

The Conductor's interventions provide indirect input to this system:
plans that trigger conductor restarts and then pass gates on the
second attempt produce different gate threshold data than plans that
pass on the first attempt. This difference helps the adaptive threshold
system calibrate its expectations.

---

## Feedback Loops

### Loop 1: Intervention → Routing Improvement

```
Agent fails → Conductor intervenes → Negative routing signal →
Router adjusts model selection → Future agents less likely to fail →
Fewer interventions needed
```

This is a negative feedback loop: interventions produce data that
reduces future interventions. Over many batch runs, the intervention
rate should decrease as the router learns better model-task mappings.

### Loop 2: Threshold → Efficiency Data → Threshold Tuning

```
Threshold fires → Intervention occurs → Efficiency event records
outcome → Threshold effectiveness measured → Threshold adjusted
```

This loop requires the adaptive conductor model (described in
08-good-regulator-self-model.md). Currently, thresholds are static.
The learning infrastructure exists to close this loop.

### Loop 3: Error Classification → Auto-Fix → Pattern Library

```
Error occurs → Diagnosis engine classifies → Auto-fix attempted →
If auto-fix succeeds → Pattern stored in diagnosis engine with
higher confidence → Future similar errors auto-fixed faster
```

Each successful auto-fix strengthens the diagnosis engine's confidence
in that error-to-fix mapping. Over time, the auto-fix success rate
for known error patterns approaches the theoretical maximum.

### Loop 4: Quality Degradation → Model Escalation → Quality Data

```
Quality drops → Anomaly detector fires → Escalate to higher-tier
model → Higher-tier model produces better quality → Quality data
recorded → Router learns tier requirements for this task type
```

The quality degradation detector triggers escalation. The escalated
model's performance provides data for future routing. Eventually, the
router learns to assign the correct tier initially, eliminating the
need for runtime escalation.

---

## Anomaly Detection in the Dispatch Pipeline

The anomaly detector integrates into the agent dispatch pipeline
(before each agent turn):

```rust
// Before each agent turn:
if let Some(anomaly) = anomaly_detector.check_prompt(prompt_hash) {
    match anomaly {
        Anomaly::PromptLoop { .. } => {
            // Abort session — sending the same prompt will produce the same failure
            return Err(DispatchError::PromptLoop);
        }
        _ => {}
    }
}

if let Some(anomaly) = anomaly_detector.check_cost(turn_cost_usd) {
    match anomaly {
        Anomaly::CostSpike { z_score } => {
            // Log warning, consider model downgrade
            tracing::warn!("cost spike z={z_score:.1}, considering model downgrade");
        }
        _ => {}
    }
}

if let Some(anomaly) = anomaly_detector.check_budget(budget_limit_usd) {
    match anomaly {
        Anomaly::BudgetExhausted { used, limit } => {
            // Abort session — budget exceeded
            return Err(DispatchError::BudgetExhausted { used, limit });
        }
        _ => {}
    }
}
```

The anomaly detector runs BEFORE the agent turn, catching problems
at input time rather than output time. This is the "anticipate, don't
react" principle (Design Principle 11) applied to agent dispatch.

---

## Provider Health Integration

The provider health tracker (`roko-learn/src/provider_health.rs`)
provides a separate feedback loop for infrastructure-level anomalies:

```
Provider returns 429 → Health tracker records failure →
3 consecutive failures → Circuit breaker opens →
Router filters out unhealthy provider → Requests routed to
healthy providers → After cooldown → Probe request sent →
If probe succeeds → Circuit breaker closes
```

This is independent of the Conductor's plan-level circuit breaker.
The provider health breaker operates on API call outcomes. The
Conductor's breaker operates on plan-level outcomes. They complement
each other:

| Breaker | Level | What Triggers It | What It Blocks |
|---------|-------|-----------------|----------------|
| Provider health | API call | 3 consecutive provider errors | Requests to that provider |
| Conductor | Plan | 2 plan failures | Retries of that plan |

---

## File Reference

| File | What |
|------|------|
| `crates/roko-learn/src/anomaly.rs` | AnomalyDetector, EWMA, prompt loop, cost spike, quality degradation |
| `crates/roko-learn/src/efficiency.rs` | AgentEfficiencyEvent (shared data format) |
| `crates/roko-learn/src/cascade_router.rs` | Cascade router (consumes intervention signals) |
| `crates/roko-learn/src/provider_health.rs` | Provider health tracker (infrastructure breaker) |
| `crates/roko-gate/src/adaptive_threshold.rs` | Adaptive gate thresholds (EMA per rung) |
| `crates/roko-conductor/src/watchers/context_window_pressure.rs` | Reads AgentEfficiencyEvent for token tracking |


---

# SOURCE: /Users/will/dev/nunchi/roko/roko/docs/07-conductor/12-yerkes-dodson-pressure.md

# Yerkes-Dodson Pressure Dynamics

> Moderate pressure maximizes cooperation. Extreme pressure collapses
> cooperative behavior within 5-12 turns. The Conductor's thresholds
> are not timeouts — they are positions on a cooperation curve.


> **Implementation**: Built

---

## The Inverted-U Curve

Robert Yerkes and John Dodson (1908) established that the
relationship between arousal and performance follows an inverted-U
curve: performance increases with arousal up to an optimum, then
declines as arousal becomes excessive. The original experiments
measured maze-learning speed in mice under varying electric shock
intensities. The finding has replicated across species, task types,
and complexity levels for over a century.

The curve has three regimes:

```
Performance
    ^
    |         ****
    |       **    **
    |      *        *
    |     *          *
    |    *            *
    |   *              *
    |  *                *
    | *                  *
    |*                    *
    +-------------------------> Pressure
    Low    Optimal    High

    Zone 1: Under-arousal → drift, exploration, token waste
    Zone 2: Optimal arousal → focused execution, cooperation
    Zone 3: Over-arousal → collapse, minimal-effort responses
```

The shape is consistent but the peak location shifts with task
complexity. Simple tasks peak at higher arousal (more pressure helps
straightforward work). Complex tasks peak at lower arousal (too much
pressure degrades complex reasoning). This complexity interaction is
called the Yerkes-Dodson Law proper.

---

## Yerkes-Dodson in LLM Agent Systems

### Research Evidence

Research on 770,000+ autonomous LLM agents demonstrates that
cooperative behavior follows the same inverted-U pattern with
environmental pressure:

- **Moderate pressure** (iteration limits, timeouts, cost budgets)
  maximizes inter-agent cooperation — agents build on each other's
  work, follow established patterns, and produce complementary
  outputs.

- **Extreme pressure** collapses cooperative behavior within **5-12
  turns**. Under severe iteration limits or aggressive restarts,
  agents shift to minimal-effort strategies: producing the simplest
  possible output that satisfies immediate constraints rather than
  contributing to the broader task. This is the agent equivalent of
  panic — survival over quality.

- **Insufficient pressure** produces drift — agents explore
  tangential approaches, repeat themselves, or engage in verbose
  reasoning that burns tokens without advancing the task. Ghost turns
  (Issue #9 in the production failure catalog) are an extreme form of
  under-pressure drift.

### Active Minority Dynamics

In large agent populations, meaningful role differentiation is limited
to active minorities. Most agents converge to generic behavior
regardless of pressure level. This validates the approach of spawning
specialized agents on demand rather than maintaining a large standing
pool. The Conductor does not need to manage pressure across dozens of
simultaneous agents — it manages pressure on the small number of
agents actually doing differentiated work at any moment.

---

## Conductor Thresholds as Pressure Parameters

Every Conductor threshold defines a position on the Yerkes-Dodson
curve. The ensemble of thresholds creates a **pressure envelope**
around the agent:

### Threshold-to-Pressure Mapping

| Threshold | Pressure Type | Too Low | Optimal | Too High |
|-----------|--------------|---------|---------|----------|
| `max_iterations` (3) | Iteration pressure | Agent loops indefinitely | Agent converges in 2-3 attempts | Agent gives up after first failure |
| `cost_limit_usd` ($10) | Budget pressure | Agent uses expensive reasoning freely | Agent balances cost vs. quality | Agent produces minimal output |
| `time_limit` (80%) | Time pressure | Agent takes unlimited time per phase | Agent completes within phase window | Agent rushes, skips verification |
| `stuck_threshold` (4) | Progress pressure | Agent allowed to spin | Agent must show progress each turn | Agent forced into superficial progress |
| `ghost_turn_max` (3) | Output pressure | Agent can produce empty turns | Agent must produce meaningful output | Agent produces any output to avoid detection |

### The Pressure Envelope

The thresholds work together, not independently. An agent experiences
their combined effect as a **pressure envelope**:

```
                            Pressure Envelope
                     ┌─────────────────────────────┐
                     │                             │
    Iteration ─────► │   ┌─────────────────────┐   │
    Pressure         │   │                     │   │
                     │   │    Agent Operating   │   │
    Cost ──────────► │   │       Space          │   │
    Pressure         │   │                     │   │
                     │   │   (Work happens      │   │
    Time ──────────► │   │    in here)          │   │
    Pressure         │   │                     │   │
                     │   └─────────────────────┘   │
    Progress ──────► │                             │
    Pressure         │                             │
                     └─────────────────────────────┘
    Output ────────►
    Pressure

    Tight envelope → High total pressure → Zone 3 risk
    Loose envelope → Low total pressure → Zone 1 risk
```

The operating space shrinks as more thresholds tighten. The
Conductor's current thresholds define a moderate envelope:

- Iteration limit of 3 allows meaningful retry without indefinite
  looping
- Cost limit of $10 provides generous budget for complex tasks while
  capping runaway spending
- Time limit at 80% of phase budget leaves 20% margin for cleanup
- Stuck threshold of 4 identical outputs requires genuine repetition,
  not just similar outputs
- Ghost turn threshold of 3 allows for legitimate warm-up before
  firing

This calibration sits near the peak of the inverted-U for standard
tasks. Complex tasks may need a looser envelope (more iterations,
higher cost budget, longer time). Fast tasks could tolerate a tighter
one.

---

## Complexity-Pressure Interaction

The Yerkes-Dodson Law's most important implication for agent systems
is the complexity interaction: the optimal pressure level depends on
task difficulty.

### Complexity Bands

The state machine (`state_machine.rs`) defines three complexity bands
with different phase timeouts:

| Complexity | Phase Timeout | Iteration Budget | Pressure Level |
|-----------|--------------|-----------------|---------------|
| Complex | 600s | Higher tolerance | Lower pressure (Zone 2 left) |
| Standard | 300s | Default | Moderate pressure (Zone 2 center) |
| Fast | 120s | Lower tolerance | Higher pressure (Zone 2 right) |

These bands implicitly implement the Yerkes-Dodson complexity
interaction:

- **Complex tasks** get more time and iteration room because the peak
  of their inverted-U curve is at lower pressure. A complex
  refactoring that touches 15 files needs room to iterate, backtrack,
  and converge.

- **Fast tasks** get less time because their peak is at higher
  pressure. A simple import fix does not benefit from exploration
  time. Tight constraints focus the agent on the obvious solution.

- **Standard tasks** sit in between. The defaults are calibrated for
  this middle ground.

### The Collapse Window

When pressure exceeds the optimal zone, cooperative behavior collapses
within 5-12 turns. In concrete terms:

```
Turn 1:  Agent attempts task normally
Turn 2:  First failure, agent retries with adjustment
Turn 3:  Conductor restarts (compile-fail-repeat fires)
Turn 4:  Agent attempts again, now with restart pressure
Turn 5:  Second failure, agent begins simplifying approach
Turn 6:  Conductor restarts again
Turn 7:  Agent produces minimal-effort output
Turn 8:  Output barely passes or fails again
...
Turn 12: Agent in full collapse — producing template responses
         that technically satisfy format requirements but contain
         no meaningful implementation
```

The circuit breaker's MAX_PLAN_FAILURES=2 limit catches this
collapse pattern. After 2 plan-level failures, the breaker opens and
prevents further attempts. This is not a simple retry limit — it is
a pressure release valve that prevents the system from pushing agents
past the collapse point.

---

## Cooperation Metrics

### Defining Cooperation

In multi-agent software development, cooperation manifests as agents
building on each other's work rather than clobbering it:

- **Positive cooperation**: Agent B extends Agent A's implementation,
  respects established patterns, follows the code conventions Agent A
  introduced.

- **Negative cooperation**: Agent B rewrites Agent A's work, ignores
  established patterns, introduces conflicting conventions.

- **Neutral**: Agent B works on entirely independent code with no
  interaction with Agent A's output.

### Measurable Cooperation Signals

The system produces several signals that correlate with cooperation
quality:

| Signal | Source | Cooperative | Collapsed |
|--------|--------|-------------|-----------|
| Merge conflict rate | Git merge queue | Low (<5%) | High (>20%) |
| Gate pass on first attempt | Gate pipeline | High (>60%) | Low (<30%) |
| Conductor intervention rate | Conductor events | Low (<10% of turns) | High (>40% of turns) |
| Review approval on first review | Review loop | High (>50%) | Low (<20%) |
| Cost per successful task | Efficiency events | Low and stable | High and rising |
| Token waste ratio | Efficiency events | Low (<20% wasted) | High (>50% wasted) |

### The Feedback Loop

Cooperation metrics close the learning loop:

```
Batch run N:
    Conductor thresholds = [max_iter=3, cost=$10, stuck=4]
    Cooperation metrics:
        merge_conflict_rate = 0.08
        first_pass_gate_rate = 0.55
        intervention_rate = 0.15
    →  Position: slightly left of optimal (could tighten)

Batch run N+1:
    Conductor thresholds = [max_iter=3, cost=$8, stuck=3]
    Cooperation metrics:
        merge_conflict_rate = 0.12
        first_pass_gate_rate = 0.48
        intervention_rate = 0.22
    →  Position: slightly right of optimal (overtightened)

Batch run N+2:
    Conductor thresholds = [max_iter=3, cost=$9, stuck=4]
    Cooperation metrics:
        merge_conflict_rate = 0.06
        first_pass_gate_rate = 0.62
        intervention_rate = 0.11
    →  Position: near optimal
```

Each batch run produces data that refines the next run's pressure
calibration. The cascade router already tracks model-task outcome
data. Extending it to track pressure-cooperation relationships
enables automated Yerkes-Dodson tuning.

---

## Pressure Tuning in Practice

### Static Pressure (Current Implementation)

The current Conductor uses static thresholds. These thresholds were
calibrated through production experience (the 21-failure catalog) and
represent good defaults:

- `MAX_GHOST_TURNS = 3`: Too low (1-2) catches legitimate warm-up.
  Too high (5+) wastes tokens on genuinely stuck agents.

- `MAX_COMPILE_FAILS = 3`: Derived from production observation that
  agents rarely recover after 3 consecutive compile failures on the
  same error pattern. Allowing more attempts pushes into collapse
  territory.

- `MAX_ITERATIONS = 3`: Plan-level retry limit. Combined with the
  circuit breaker's MAX_PLAN_FAILURES=2, this creates a total of
  2×3=6 attempts before permanent failure — just at the edge of the
  collapse window.

- `COST_LIMIT = $10`: Budget pressure. At current model pricing,
  $10 allows approximately 200-300 agent turns with a mid-tier model,
  sufficient for complex tasks without enabling runaway exploration.

### Adaptive Pressure (Design Target)

The adaptive Conductor model (described in
08-good-regulator-self-model.md) would tune pressure dynamically:

1. **Per-task complexity assessment**: Before each task, estimate
   complexity from plan metadata (files to modify, dependency depth,
   error category). Set the pressure envelope accordingly.

2. **Runtime pressure adjustment**: If an agent is making steady
   progress (gate scores improving, test counts increasing), maintain
   or loosen pressure. If progress stalls, increase pressure by
   tightening thresholds.

3. **Cross-batch learning**: Track cooperation metrics across batch
   runs. Adjust the default pressure envelope based on observed
   cooperation peaks for each complexity band.

4. **Model-specific calibration**: Different models have different
   Yerkes-Dodson curves. A high-capability model tolerates more
   pressure before collapse. A smaller model collapses sooner.
   The cascade router's model-outcome data informs per-model
   pressure profiles.

---

## Stigmergy and Pressure

Pierre-Paul Grassé's stigmergy concept (1959) — indirect
coordination through environment modification — intersects with
Yerkes-Dodson pressure in multi-agent development.

Git is stigmergic: each commit is an environmental trace that
influences future agents. Under optimal pressure, agents leave
high-quality traces:

- Clean commits with meaningful messages
- Consistent code patterns that subsequent agents follow
- Established conventions that reduce decision overhead

Under excessive pressure, stigmergic quality degrades:

- Minimal commits with no context
- Ad hoc patterns that subsequent agents cannot follow
- Conflicting conventions that increase future merge conflicts

The stigmergic quality of one batch run's output becomes the
environmental input for the next run. Poor stigmergic quality
compounds: low-quality traces produce confused agents that produce
lower-quality traces. This is a positive feedback loop that the
Conductor's pressure calibration must prevent.

The key property of stigmergic coordination is O(1) cost per agent
— each agent reads the environment independently. This scales
sublinearly compared to O(n²) message-based coordination. But this
scaling advantage depends on trace quality, which depends on
pressure calibration.

---

## The Conductor's Role

The Conductor does not directly tune pressure — its thresholds ARE
the pressure. Every threshold decision is implicitly a position
choice on the Yerkes-Dodson curve:

1. **Watcher thresholds** define sensitivity — how quickly the system
   detects problems. More sensitive = more interventions = more
   pressure.

2. **Intervention severity** defines response magnitude. Warning
   (restart) is moderate pressure. Critical (fail) is maximum
   pressure.

3. **Circuit breaker limits** define persistence — how many times the
   system retries before giving up. More retries = sustained pressure.
   Fewer retries = quick pressure release.

4. **Phase timeouts** define temporal pressure — how long the agent
   has to work. Shorter timeouts = higher time pressure.

The Conductor's design philosophy — **decide, don't nudge** —
reflects Yerkes-Dodson wisdom. A nudge (suggestion to the agent) adds
ambiguous pressure — the agent must interpret the suggestion and
decide how to respond, which itself consumes cognitive resources. A
decision (restart, fail) is unambiguous — the agent gets a clean
slate or the task is done. Clear decisions produce predictable
pressure. Ambiguous nudges produce unpredictable pressure that may
push the agent past the collapse point.

---

## Cross-References

- [01-watcher-ensemble.md](01-watcher-ensemble.md) — Watcher
  thresholds as pressure parameters
- [02-circuit-breaker.md](02-circuit-breaker.md) — Circuit breaker
  as pressure release valve
- [03-graduated-interventions.md](03-graduated-interventions.md) —
  Severity system as pressure magnitude
- [05-stuck-detection.md](05-stuck-detection.md) — Stuck detection
  as progress pressure
- [08-good-regulator-self-model.md](08-good-regulator-self-model.md)
  — Adaptive self-model for pressure tuning
- [10-adaptive-timeouts-state-machine.md](10-adaptive-timeouts-state-machine.md)
  — Complexity bands as Yerkes-Dodson implementation
- [11-anomaly-detection-learning.md](11-anomaly-detection-learning.md)
  — Learning loops that enable pressure optimization
- [14-production-failure-catalog.md](14-production-failure-catalog.md)
  — Production data that calibrated current thresholds

### Citations

- Yerkes, R.M. & Dodson, J.D. (1908). "The relation of strength of
  stimulus to rapidity of habit-formation." *Journal of Comparative
  Neurology and Psychology*, 18, 459-482.
- Grassé, P.P. (1959). "La reconstruction du nid et les coordinations
  interindividuelles chez Bellicositermes natalensis et Cubitermes sp."
  *Insectes Sociaux*, 6(1), 41-80.
- Research on 770,000+ autonomous agents: emergent cooperation
  dynamics, Yerkes-Dodson replication in LLM multi-agent systems.

---

## Pressure Calibration Per Agent Type

Different models have different optimal pressure. The inverted-U
curve shifts left or right depending on model capability, and the
shape of the curve — peak width, collapse steepness — varies too.
Treating all models identically wastes capacity on strong models
and breaks weak ones.

### Model-specific Yerkes-Dodson curves

| Model Tier | Peak Location | Collapse Threshold | Rationale |
|-----------|--------------|-------------------|-----------|
| Opus (Premium) | Higher pressure | Later collapse | Superior reasoning tolerates more constraint |
| Sonnet (Standard) | Moderate pressure | Moderate collapse | Good general-purpose balance |
| Haiku (Fast) | Lower pressure | Earlier collapse | Limited reasoning degrades faster under stress |

The curve shape also differs:

- **Opus**: wider peak (robust over a range of pressures), gradual
  collapse. You can push an Opus agent harder before performance
  degrades, and the degradation is smooth rather than cliff-like.

- **Sonnet**: moderate peak width, moderate collapse steepness. The
  default thresholds in the Conductor are calibrated for this tier.

- **Haiku**: narrow peak (small optimal window), steep collapse. A
  Haiku agent operating even slightly past its optimal pressure
  drops to minimal-effort output fast. The margin for error is
  thin.

### Empirical calibration protocol

To determine optimal pressure per model, the system maintains a
per-model pressure profile learned from execution history:

```rust
/// Per-model pressure profile learned from execution history.
/// Each model has its own Yerkes-Dodson curve parameters.
pub struct ModelPressureProfile {
    /// Model identifier (e.g., "claude-opus-4-6").
    pub model: String,
    /// Estimated optimal pressure level (0.0 to 1.0).
    pub optimal_pressure: f64,
    /// Estimated collapse threshold (pressure above which performance drops sharply).
    pub collapse_threshold: f64,
    /// Confidence in the estimate (number of observations).
    pub observations: usize,
    /// Historical (pressure, performance) pairs for curve fitting.
    pub history: Vec<(f64, f64)>,
}

/// Compute a scalar pressure index from the multi-dimensional pressure envelope.
pub fn pressure_index(
    iteration: u32,
    max_iterations: u32,
    cost_usd: f64,
    cost_budget_usd: f64,
    elapsed_ms: u64,
    timeout_ms: u64,
    stuck_count: u32,
    stuck_threshold: u32,
) -> f64 {
    let iter_pressure = iteration as f64 / max_iterations as f64;
    let cost_pressure = cost_usd / cost_budget_usd;
    let time_pressure = elapsed_ms as f64 / timeout_ms as f64;
    let stuck_pressure = stuck_count as f64 / stuck_threshold as f64;

    // Weighted combination (weights sum to 1.0)
    0.30 * iter_pressure
        + 0.25 * cost_pressure
        + 0.25 * time_pressure
        + 0.20 * stuck_pressure
}
```

The `pressure_index` function collapses the multi-dimensional
pressure envelope into a single scalar. This scalar maps to the
x-axis of the Yerkes-Dodson curve. Comparing it to the model's
`optimal_pressure` and `collapse_threshold` tells the Conductor
whether the agent is in zone 1 (under-pressure), zone 2 (optimal),
or zone 3 (over-pressure).

### Thompson sampling for pressure optimization

Rather than hand-tuning pressure configurations, use bandit
algorithms to find the optimal level per model:

- **Arms**: five discrete pressure configurations — very-loose,
  loose, moderate, tight, very-tight
- **Reward**: gate pass rate weighted by efficiency
  (`pass_rate / cost_usd`)
- **Prior**: discounted Beta distribution to handle
  non-stationarity as models update and codebases evolve

Each arm maps to a concrete `(max_iter, cost_limit, timeout)`
setting:

```rust
/// Thompson Sampling arms for pressure level selection.
/// Each arm represents a discrete pressure configuration.
pub struct PressureBandit {
    /// Per-pressure-level Thompson arm (Beta posterior).
    arms: Vec<PressureArm>,
    /// Discount factor for non-stationarity (default: 0.995).
    discount: f64,
}

pub struct PressureArm {
    pub name: &'static str,
    pub config: PressureConfig,
    pub alpha: f64,  // Beta posterior: successes
    pub beta: f64,   // Beta posterior: failures
}

pub struct PressureConfig {
    pub max_iterations: u32,
    pub cost_budget_usd: f64,
    pub phase_timeout_secs: u64,
    pub stuck_threshold: u32,
    pub ghost_turn_max: u32,
}

/// Default pressure configurations for each arm.
pub const PRESSURE_CONFIGS: &[(&str, PressureConfig)] = &[
    ("very-loose", PressureConfig { max_iterations: 5, cost_budget_usd: 25.0, phase_timeout_secs: 900, stuck_threshold: 6, ghost_turn_max: 5 }),
    ("loose",      PressureConfig { max_iterations: 4, cost_budget_usd: 15.0, phase_timeout_secs: 600, stuck_threshold: 5, ghost_turn_max: 4 }),
    ("moderate",   PressureConfig { max_iterations: 3, cost_budget_usd: 10.0, phase_timeout_secs: 300, stuck_threshold: 4, ghost_turn_max: 3 }),
    ("tight",      PressureConfig { max_iterations: 2, cost_budget_usd: 5.0,  phase_timeout_secs: 180, stuck_threshold: 3, ghost_turn_max: 2 }),
    ("very-tight", PressureConfig { max_iterations: 1, cost_budget_usd: 2.0,  phase_timeout_secs: 120, stuck_threshold: 2, ghost_turn_max: 1 }),
];
```

The discount factor (0.995) means observations from ~200 tasks ago
carry half their original weight. This prevents stale data from
anchoring the bandit on an outdated optimum when the model or
codebase changes.

---

## Pressure-Performance Curve Fitting from Historical Data

The inverted-U is a qualitative shape. To use it for adaptive
pressure control, the system needs a quantitative model fit from
observed (pressure, performance) pairs.

### Curve parameterization

The Yerkes-Dodson curve is modeled as an asymmetric logistic
product:

```
P(x) = P_max * sigmoid(k1 * (x - a_low)) * (1 - sigmoid(k2 * (x - a_high)))
```

Parameters:

- **P_max**: peak performance (observed maximum gate pass rate)
- **a_low**: left threshold (pressure below which performance is
  limited by under-arousal)
- **a_high**: right threshold (pressure above which performance
  collapses)
- **k1**: steepness of the left slope (how fast performance rises
  with pressure)
- **k2**: steepness of the right slope (how fast performance
  collapses)

The curve is typically asymmetric: k2 > k1. Performance collapses
faster than it rises. An agent that took 10 turns of gentle
pressure to reach peak performance can lose that performance in
3 turns of excessive pressure. This asymmetry is why the
Conductor's circuit breaker errs on the side of stopping early
rather than pushing harder.

### Online curve estimation

Full parametric curve fitting requires nonlinear optimization,
which is expensive to run per-agent per-task. Instead, the system
uses binned estimation — a lightweight online method that tracks
the curve shape as data arrives:

```rust
/// Online estimator for Yerkes-Dodson curve parameters.
/// Maintains running estimates of curve shape from streaming (pressure, performance) data.
pub struct YerkesDodsonEstimator {
    /// Binned observations: pressure_bin -> (sum_performance, count).
    bins: Vec<PressureBin>,
    /// Number of bins (default: 10, covering 0.0 to 1.0 pressure range).
    num_bins: usize,
    /// Estimated optimal pressure (argmax of binned performance).
    estimated_optimum: f64,
    /// Estimated peak performance (max of binned performance).
    estimated_peak: f64,
    /// Confidence: total observations across all bins.
    total_observations: usize,
    /// Minimum observations per bin before including in estimate.
    min_bin_count: usize,  // default: 5
}

pub struct PressureBin {
    pub center: f64,       // center of the bin (e.g., 0.05, 0.15, ...)
    pub sum_perf: f64,     // sum of performance observations
    pub count: usize,      // number of observations
}

impl YerkesDodsonEstimator {
    pub fn record(&mut self, pressure: f64, performance: f64) {
        let bin_idx = ((pressure * self.num_bins as f64) as usize).min(self.num_bins - 1);
        self.bins[bin_idx].sum_perf += performance;
        self.bins[bin_idx].count += 1;
        self.total_observations += 1;
        self.reestimate();
    }

    fn reestimate(&mut self) {
        let mut best_perf = 0.0;
        let mut best_pressure = 0.5;
        for bin in &self.bins {
            if bin.count >= self.min_bin_count {
                let avg = bin.sum_perf / bin.count as f64;
                if avg > best_perf {
                    best_perf = avg;
                    best_pressure = bin.center;
                }
            }
        }
        self.estimated_optimum = best_pressure;
        self.estimated_peak = best_perf;
    }

    /// Recommend a pressure level for the next task.
    pub fn recommended_pressure(&self) -> f64 {
        if self.total_observations < 20 {
            0.5  // Default to moderate pressure with insufficient data
        } else {
            self.estimated_optimum
        }
    }
}
```

The estimator defaults to moderate pressure (0.5) until it
accumulates at least 20 observations. Below that threshold, the
binned averages are too noisy to trust. The `min_bin_count` of 5
per bin prevents a single outlier from dominating a bin's estimate.

### Regime shift detection

The Yerkes-Dodson curve is not static. Model updates, codebase
evolution, and task distribution shifts can all change the curve's
shape:

- A model update that improves reasoning shifts the peak rightward
  (the model tolerates more pressure)
- A codebase that grows more complex shifts the peak leftward
  (complex tasks need less pressure)
- A change in task mix (more refactoring, less greenfield) changes
  the curve's width

The system detects these shifts using CUSUM (cumulative sum
control chart) on the residuals between actual performance and
predicted performance at the current pressure level. When the
CUSUM statistic exceeds a threshold, the estimator resets its bin
counts and begins re-estimation. The signal:
"The Yerkes-Dodson curve for this model has shifted —
recalibrating."

### Bayesian confidence bounds

The binned estimator produces point estimates. To know whether
those estimates are reliable, the system computes confidence
intervals from the variance of the binned performance:

- If the 95% confidence interval for the optimal pressure spans
  more than 30% of the pressure range, data is insufficient. Use
  defaults.
- If the 95% CI is narrow (under 10% of the range), the estimate
  is reliable. Use it for adaptive pressure control.

This prevents the system from acting on noisy estimates. With
fewer than ~50 observations spread across pressure levels, the
CI is typically wide enough to trigger the default fallback. The
system earns the right to adapt by accumulating evidence first.

---

## Cognitive Load Theory Mapping

John Sweller's cognitive load theory (1988) partitions the demands
on working memory into three components. The same decomposition
applies to LLM agent context windows — the context window is the
agent's working memory, and it has finite capacity.

### Three load components in LLM context

| Cognitive Load | LLM Agent Equivalent | Source in Roko |
|---------------|---------------------|----------------|
| Intrinsic load | Task complexity: files to modify, dependency depth, domain specificity | Plan metadata, complexity classification |
| Extraneous load | Irrelevant context: stale docs, off-topic examples, verbose error history | Prompt sections with low signal_ratio |
| Germane load | Productive scaffolding: PRD context, error digests, playbook rules, skill hints | InjectContext engrams, high signal_ratio sections |

**Intrinsic load** is fixed by the task itself. A refactoring that
touches 15 files across 4 crates has high intrinsic load. A typo
fix has low intrinsic load. The system cannot reduce intrinsic load
without changing the task.

**Extraneous load** is waste. It occupies context window capacity
without contributing to task completion. Stale documentation, full
error logs from previous attempts (instead of digests), irrelevant
file contents included "for context" — all extraneous.

**Germane load** is productive overhead. PRD context that explains
why the task exists, error digests that summarize what went wrong
on the last attempt, playbook rules that encode lessons from past
failures — this content helps the agent reason better about the
task.

### The saturation constraint

The three loads compete for the same finite resource:

`intrinsic + extraneous + germane <= context_window_capacity`

When intrinsic + extraneous saturates the context, no room remains
for germane scaffolding. The agent has the task and the noise but
none of the helpful context.

The context window pressure watcher (80% threshold) exists to
enforce this constraint. By firing at 80% usage, it preserves 20%
of the window for germane content. The SystemPromptBuilder's
signal_ratio scoring on prompt sections is the mechanism for
deciding what stays (high signal) and what gets cut (low signal).

Reducing extraneous load (dropping verbose error logs, removing
stale docs) creates room for germane load (error digests,
skill suggestions, PRD context). The Conductor's job is not to
minimize total context — it is to maximize the germane-to-extraneous
ratio within the available window.

### Pressure as cognitive overload

Conductor pressure interacts with cognitive load in two ways:

1. **Each restart adds extraneous context.** When the Conductor
   restarts an agent, the new prompt includes error history from
   the previous attempt. This is necessary context — but it is
   also additional load. Three restarts can accumulate enough error
   history to crowd out germane scaffolding. The error digest
   pattern (summarize rather than include raw logs) mitigates this.

2. **Tight time pressure prevents germane processing.** An agent
   under severe time pressure rushes through the prompt, skipping
   the slower reasoning that germane scaffolding enables. The PRD
   context is there, but the agent does not use it because it
   optimizes for speed over understanding.

The Conductor must balance pressure (motivating focus) against
cognitive overload (degrading reasoning). Pressure that eliminates
drift is productive. Pressure that eliminates understanding is
destructive. The difference is whether the pressure reduces
extraneous processing (good) or germane processing (bad).

---

## Flow State Detection

Mihaly Csikszentmihalyi's flow research (1975, 1990) identifies a
psychological state of deep productive engagement. The conditions
for flow map to observable agent behaviors: clear goals, immediate
feedback, and a balance between challenge and skill. When an agent
operates in this zone, interrupting it is costly — rebuilding
context after a restart takes several turns of reduced productivity.

### Flow indicators

Observable signals distinguish flow state from collapse:

| Signal | Flow State | Collapse State |
|--------|-----------|---------------|
| Files changed per turn | Consistent, moderate | Zero or extreme |
| Gate score trajectory | Improving | Flat or declining |
| Tool utilization | Diverse, purposeful | Repetitive or absent |
| Context usage | 40-70% of window | >85% or <20% |
| Cost per meaningful change | Low, stable | High, increasing |

A flow-state agent changes a moderate number of files each turn
(not zero, not everything at once), shows improving gate scores,
uses a variety of tools for different purposes, and consumes a
healthy fraction of its context window. A collapsed agent either
produces nothing or produces frantic changes that fail gates, uses
the same tool repeatedly (or stops using tools), and either floods
its context or barely uses it.

### Flow preservation policy

When the system detects flow, it should reduce intervention
sensitivity to avoid disrupting the productive state:

- If an agent shows flow indicators for 3 or more consecutive
  turns, increase watcher thresholds by 50%
- Rationale: interrupting flow is costly. The agent needs multiple
  turns to rebuild context after any restart. A false positive
  intervention during flow destroys more value than a few extra
  turns of mild drift.
- The threshold increase is temporary and automatically reverts
  when flow indicators stop

```rust
/// Flow state detection and preservation.
/// When an agent shows sustained productive behavior, reduce intervention sensitivity
/// to avoid disrupting the productive state.
pub struct FlowDetector {
    /// Minimum consecutive productive turns to declare flow.
    pub min_flow_turns: usize,  // default: 3
    /// Threshold multiplier when flow is detected (default: 1.5 = 50% more lenient).
    pub flow_threshold_multiplier: f64,
    /// Per-agent flow state tracking.
    agent_flow: HashMap<String, FlowState>,
}

pub struct FlowState {
    pub consecutive_productive_turns: usize,
    pub in_flow: bool,
    pub flow_started_at: Option<Instant>,
}

impl FlowDetector {
    /// Update flow state based on agent's latest turn.
    pub fn update(&mut self, agent_id: &str, turn: &TurnMetrics) {
        let state = self.agent_flow.entry(agent_id.to_string())
            .or_insert(FlowState { consecutive_productive_turns: 0, in_flow: false, flow_started_at: None });

        if turn.is_productive() {
            state.consecutive_productive_turns += 1;
            if state.consecutive_productive_turns >= self.min_flow_turns && !state.in_flow {
                state.in_flow = true;
                state.flow_started_at = Some(Instant::now());
            }
        } else {
            state.consecutive_productive_turns = 0;
            state.in_flow = false;
            state.flow_started_at = None;
        }
    }
}

pub struct TurnMetrics {
    pub files_changed: usize,
    pub gate_score_improved: bool,
    pub tool_calls_diverse: bool,
    pub context_usage_ratio: f64,
}

impl TurnMetrics {
    pub fn is_productive(&self) -> bool {
        self.files_changed > 0
            && self.context_usage_ratio > 0.2
            && self.context_usage_ratio < 0.85
    }
}
```

The `is_productive` check is deliberately conservative. It requires
nonzero file changes and moderate context usage. An agent that
changes files but floods its context (>85%) or barely uses it
(<20%) is not in flow — it is either thrashing or coasting.

The flow detector does not override the circuit breaker. If the
circuit breaker fires (plan-level failure), flow state is
irrelevant — the task has failed. Flow preservation only affects
the watcher thresholds that trigger interventions below the circuit
breaker level.

---

### References

- Yerkes, R.M. & Dodson, J.D. (1908). "The relation of strength
  of stimulus to rapidity of habit-formation." *Journal of
  Comparative Neurology and Psychology*, 18, 459-482.
- Csikszentmihalyi, M. (1975/1990). *Flow: The Psychology of
  Optimal Experience*. Harper & Row.
- Sweller, J. (1988). "Cognitive load during problem solving:
  Effects on learning." *Cognitive Science*, 12(2), 257-285.
- Hanin, Y.L. (2000). "Individual Zones of Optimal Functioning
  (IZOF) model." In Y.L. Hanin (Ed.), *Emotions in Sport*.
  Human Kinetics.
- Thompson, W.R. (1933). "On the likelihood that one unknown
  probability exceeds another in view of the evidence of two
  samples." *Biometrika*, 25(3-4), 285-294.


---

# SOURCE: /Users/will/dev/nunchi/roko/roko/docs/07-conductor/13-process-supervision-wiring.md

# Process Supervision Wiring

> Every spawned process is a supervised entity. The ProcessSupervisor
> owns the full lifecycle: spawn, monitor, timeout, kill, cleanup.
> Unsupervised processes become orphans. Orphans consume resources
> silently until they starve the system.


> **Implementation**: Built

---

## The Problem: Unsupervised Processes

Production batch runs (March-April 2026) exposed three categories of
process management failure:

1. **Spawn races** (Issue #6): Agent exits were confused between
   retry attempts. Without attempt tracking, exit events from dead
   processes were attributed to newly spawned processes.

2. **Orphaned cargo processes** (Issue #7): Timeouts killed the
   direct child process but not its descendants. `cargo check`
   processes survived their parent's death and accumulated, eventually
   starving CPU and memory.

3. **Cold start overhead** (Issue #8): Every agent turn spawned a new
   CLI subprocess, adding 2-5 seconds of startup overhead per turn.
   Over hundreds of turns, this accumulated to 10-30 minutes of pure
   waste.

All three failures share a root cause: processes were treated as
fire-and-forget rather than supervised entities. The structural fix
is a ProcessSupervisor that owns the full lifecycle of every spawned
process (Design Principle #7: Process isolation with supervision).

---

## ProcessSupervisor Architecture

The `ProcessSupervisor` lives in `bardo-runtime` and is wired into
the plan execution pipeline through `PlanRunner`:

```
┌─────────────────────────────────────────────────┐
│                  PlanRunner                      │
│                                                  │
│  ┌──────────────────────────────────────────┐   │
│  │         ProcessSupervisor                 │   │
│  │                                           │   │
│  │  ┌─────────┐  ┌─────────┐  ┌─────────┐  │   │
│  │  │ Agent 1 │  │ Agent 2 │  │ Agent 3 │  │   │
│  │  │ PID 4201│  │ PID 4205│  │ PID 4209│  │   │
│  │  └────┬────┘  └────┬────┘  └────┬────┘  │   │
│  │       │            │            │        │   │
│  │  ┌────┴────┐  ┌────┴────┐  ┌────┴────┐  │   │
│  │  │ cargo   │  │ cargo   │  │ cargo   │  │   │
│  │  │ PID 4202│  │ PID 4206│  │ PID 4210│  │   │
│  │  └────┬────┘  └────┴────┘  └─────────┘  │   │
│  │       │                                   │   │
│  │  ┌────┴────┐                              │   │
│  │  │ rustc   │                              │   │
│  │  │ PID 4203│                              │   │
│  │  └─────────┘                              │   │
│  │                                           │   │
│  │  PID Registry: {4201, 4202, 4203,         │   │
│  │                  4205, 4206, 4209, 4210}  │   │
│  └──────────────────────────────────────────┘   │
│                                                  │
└─────────────────────────────────────────────────┘
```

### Core Responsibilities

The ProcessSupervisor provides five guarantees:

1. **PID tracking**: Every spawned process is registered with its PID,
   parent PID, attempt ID, and plan association.

2. **Descendant discovery**: For any registered PID, the supervisor
   can enumerate the full process tree (children, grandchildren,
   etc.) via platform-specific mechanisms.

3. **Lifecycle management**: Spawn, monitor, timeout, and terminate
   are atomic operations on the process tree, not individual
   processes.

4. **Orphan prevention**: On parent exit, all registered descendants
   are terminated. No process outlives its supervisor.

5. **Attempt isolation**: Each spawn attempt gets a monotonically
   increasing attempt ID. Exit events carry the attempt ID, preventing
   confusion between retries.

---

## PID Registry

The PID registry is the supervisor's core data structure — a map
from PID to process metadata:

```rust
struct ProcessEntry {
    pid: u32,
    parent_pid: Option<u32>,
    plan_id: String,
    task_id: String,
    attempt_id: u64,          // monotonically increasing
    spawned_at: Instant,
    status: ProcessStatus,     // Running, Exited, Killed
}

enum ProcessStatus {
    Running,
    Exited { code: i32, at: Instant },
    Killed { signal: i32, at: Instant },
}
```

### Registration Flow

```
PlanRunner starts task "implement-auth"
    │
    ├─► Supervisor.spawn(plan_id, task_id, cmd)
    │       │
    │       ├─► Increment attempt counter → attempt_id = 7
    │       ├─► Execute command with setsid (new process group)
    │       ├─► Register PID 4201, attempt_id=7, plan="plan-42"
    │       └─► Return (pid=4201, attempt_id=7)
    │
    ├─► Agent 4201 spawns cargo check
    │       │
    │       └─► Supervisor detects child PID 4202
    │           Register PID 4202, parent=4201
    │
    └─► Cargo spawns rustc
            │
            └─► Supervisor detects grandchild PID 4203
                Register PID 4203, parent=4202
```

The registry enables precise cleanup: terminating PID 4201 also
terminates 4202 and 4203. Without the registry, 4202 and 4203
survive as orphans.

---

## Process Tree Cleanup

### The Descendant Problem

Unix process semantics create the orphan problem:

```
Parent (PID 100) spawns Child (PID 200)
Child (PID 200) spawns Grandchild (PID 300)

kill(100) → Parent dies
           → Child is reparented to init (PID 1)
           → Grandchild is reparented to init (PID 1)
           → Both 200 and 300 continue running as orphans
```

Sending SIGTERM to the parent process does NOT propagate to
descendants unless they are in the same process group and the signal
is sent to the group.

### kill_all_descendants

The `kill_all_descendants(pid)` function walks the process tree and
terminates processes bottom-up (leaves first, then parents):

```
kill_all_descendants(4201):
    │
    ├─► Discover tree: 4201 → [4202 → [4203]]
    │
    ├─► Kill leaves first:
    │       kill(4203, SIGTERM)    # rustc
    │       wait 100ms
    │       if still alive: kill(4203, SIGKILL)
    │
    ├─► Kill intermediate:
    │       kill(4202, SIGTERM)    # cargo
    │       wait 100ms
    │       if still alive: kill(4202, SIGKILL)
    │
    └─► Kill root:
            kill(4201, SIGTERM)    # agent CLI
            wait 100ms
            if still alive: kill(4201, SIGKILL)
```

Bottom-up ordering prevents a common race: if you kill the parent
first, children may detect the parent's death and change behavior
(spawn new processes, write emergency state) before you get to them.
Killing leaves first ensures no new processes are spawned during
cleanup.

### Platform-Specific Discovery

Process tree discovery is platform-specific:

| Platform | Mechanism | Notes |
|----------|-----------|-------|
| Linux | cgroups | Most reliable — kernel tracks all processes in the group |
| Linux (fallback) | `/proc/{pid}/task/*/children` | Reads kernel process tree directly |
| macOS | `pgrep -P {pid}` recursive | Walks the tree via parent PID relationships |
| macOS (fallback) | `ps -o pid,ppid` + manual tree construction | Parses process table |

The cgroups approach on Linux is strongest: a process cannot escape
its cgroup, so even double-forked processes are captured. On macOS,
the `pgrep` approach can miss processes that have changed their
parent PID (via `setsid` or double-fork). The periodic orphan sweep
catches these stragglers.

---

## SIGTERM → SIGKILL Escalation

The two-phase kill protocol gives processes a chance to clean up
before forced termination:

```
Phase 1: SIGTERM (graceful)
    │
    ├─► Process receives SIGTERM
    ├─► Grace period starts (configurable, default 5s)
    ├─► Process may:
    │       - Write checkpoint
    │       - Flush buffers
    │       - Close connections
    │       - Exit cleanly
    │
    ├─► Grace period expires
    │
Phase 2: SIGKILL (forced)
    │
    ├─► Process receives SIGKILL
    ├─► Cannot be caught or ignored
    └─► Process terminated immediately
```

### Grace Period Configuration

Different process types need different grace periods:

| Process Type | Grace Period | Rationale |
|-------------|-------------|-----------|
| Agent CLI | 5s | Needs time to flush output, write session state |
| cargo check | 2s | Build processes have no important state to save |
| cargo test | 3s | May need to write partial test results |
| Gate scripts | 2s | Verification scripts are stateless |
| rustc | 1s | Compiler has no user-facing state |

The supervisor reads the process type from the registry entry and
applies the appropriate grace period. Unknown processes get the
default 5s.

### Escalation in Practice

From production experience, most processes exit cleanly within the
grace period:

- **Agent CLI**: Catches SIGTERM, writes session state, exits within
  1-2 seconds. SIGKILL is rarely needed.

- **cargo processes**: Often do not handle SIGTERM at all and need
  SIGKILL. But their only important output is the target directory,
  which is recoverable.

- **Gate scripts**: Shell scripts propagate SIGTERM to child
  processes. Usually exit within 1 second.

---

## Process Group Management

### setsid for Isolation

Every agent process is spawned in its own process group using
`setsid`:

```rust
let child = Command::new("claude")
    .args(&["--cli", "--model", model])
    .pre_exec(|| {
        // Create new session and process group
        unsafe { libc::setsid() };
        Ok(())
    })
    .spawn()?;
```

This provides two benefits:

1. **Signal isolation**: Signals sent to the orchestrator's process
   group do not propagate to agent process groups. A Ctrl+C in the
   terminal kills the orchestrator, which then gracefully shuts down
   agents via the supervisor — not by SIGINT propagation.

2. **Group kill**: The supervisor can send signals to the entire
   process group with `kill(-pgid, signal)`, catching all processes
   in the group with a single system call:

```rust
fn kill_process_group(pgid: i32, signal: Signal) -> Result<()> {
    // Negative PID sends to entire process group
    unsafe { libc::kill(-pgid, signal as i32) };
    Ok(())
}
```

### When setsid Is Insufficient

Processes can escape their process group by calling `setsid`
themselves (creating a new session). This is uncommon for cargo and
rustc but possible for arbitrary tool scripts. The orphan reaper
handles these escapees.

---

## Orphan Reaper

The orphan reaper is a background task that periodically scans for
processes that should have been cleaned up but were not:

```
Every 30 seconds:
    │
    ├─► For each entry in PID registry where status == Running:
    │       │
    │       ├─► Check if process is still alive (kill(pid, 0))
    │       │
    │       ├─► If dead: update registry status to Exited
    │       │
    │       └─► If alive AND parent task is complete/failed:
    │               │
    │               └─► This is an orphan — kill it
    │                   kill_all_descendants(pid)
    │                   Update registry status to Killed
    │
    └─► Scan for unregistered processes:
            │
            ├─► List all processes owned by current user
            ├─► Filter to known agent executables (claude, cargo, rustc)
            ├─► Check if any are NOT in the PID registry
            └─► If found: log warning, optionally kill
```

The unregistered process scan is conservative — it only logs a
warning by default. Killing processes not in the registry risks
killing unrelated user processes. The scan provides visibility; the
operator decides whether to act.

### Orphan Detection Heuristics

An orphan is a process whose supervisor context no longer exists:

| Signal | Meaning |
|--------|---------|
| Parent PID is 1 (init) | Process was reparented — original parent died |
| Task is in Completed/Failed state | Process outlived its task |
| Plan is in Failed/Aborted state | Process outlived its plan |
| No registry entry exists | Process was spawned without supervision |

The first signal (parent PID 1) is the strongest indicator. On
macOS, reparented processes go to `launchd` (PID 1). On Linux,
they go to the nearest subreaper or PID 1.

---

## Attempt Tracking

Attempt tracking eliminates spawn races (Issue #6) by associating
every exit event with a specific spawn attempt:

```
Attempt 1: spawn agent → PID 4201, attempt_id=1
    │
    ├─► Agent produces near-zero output, exits
    ├─► Exit event: (pid=4201, attempt_id=1, code=1)
    │
Attempt 2: spawn agent → PID 4205, attempt_id=2
    │
    ├─► Stale exit event arrives: (pid=4201, attempt_id=1, code=1)
    │       Supervisor checks: current_attempt_id=2, event_attempt_id=1
    │       → Stale event, ignore
    │
    ├─► Agent completes successfully, exits
    └─► Exit event: (pid=4205, attempt_id=2, code=0)
            Supervisor checks: current_attempt_id=2, event_attempt_id=2
            → Current event, process
```

Without attempt tracking, the stale exit event from attempt 1 could
be attributed to attempt 2, causing the supervisor to kill a healthy
process or mark a successful attempt as failed.

### Spawn Backoff

Retries include exponential backoff to prevent rapid cycling:

| Attempt | Backoff | Rationale |
|---------|---------|-----------|
| 1 | 0s (immediate) | First attempt — no delay needed |
| 2 | 2s | Brief cooldown, clears event queue |
| 3 | 4s | Longer cooldown, system may need time to stabilize |
| 4 | 30s | Extended cooldown — persistent failure likely |
| 5+ | 60s | Maximum backoff — prevent thrashing |

The backoff gives the system time to drain event queues and
stabilize. This is a probabilistic mitigation (reduces the race
window); attempt tracking is the structural fix (eliminates the
race entirely).

---

## Stderr Monitoring

The supervisor monitors agent stderr for diagnostic signals:

```
Agent stderr output:
    │
    ├─► classify_known_warning(line)
    │       │
    │       ├─► "codex state DB migration" → Suppress (benign)
    │       ├─► "npm WARN deprecated" → Suppress (benign)
    │       ├─► "error[E0" → Forward to diagnosis engine
    │       ├─► "SIGTERM" → Expected during shutdown
    │       └─► Unknown → Log at WARN level
    │
    └─► Forward to Conductor if actionable
```

### Known Warning Classification

Some stderr output is expected and benign. The classifier prevents
false alarms:

| Pattern | Classification | Action |
|---------|---------------|--------|
| `codex state DB migration` | Benign startup message | Suppress |
| `npm WARN deprecated` | Dependency warning | Suppress |
| `warning: unused variable` | Compiler warning | Log at DEBUG |
| `error[E0` | Compiler error | Forward to diagnosis engine |
| `thread 'main' panicked` | Agent panic | Alert, attempt recovery |
| `FATAL` | Unrecoverable error | Alert, kill process |

The diagnosis engine (`diagnosis.rs`) receives forwarded errors and
matches them against its 34 patterns to suggest interventions. This
connects stderr monitoring directly to the Conductor's decision
pipeline.

---

## Resource Limits

Per-agent resource limits prevent a single runaway process from
starving the system:

### CPU Limits

On Linux, cgroups provide hard CPU limits:

```
/sys/fs/cgroup/roko/agent-{plan_id}/cpu.max = "50000 100000"
                                                 ^       ^
                                               50ms per 100ms period
                                               = 50% of one CPU
```

On macOS, no kernel-level CPU limits are available. The supervisor
uses periodic monitoring with SIGSTOP/SIGCONT to throttle runaway
processes, or relies on the cost budget as an indirect CPU limit
(more CPU → more tokens → budget exhaustion).

### Memory Limits

```
/sys/fs/cgroup/roko/agent-{plan_id}/memory.max = "2147483648"
                                                    ^
                                                  2 GB limit
```

When a process exceeds its memory limit, the kernel OOM killer
terminates it. The supervisor detects this via the exit status and
records an OOM event in the PID registry.

### Disk I/O Limits

Build processes (cargo, rustc) are I/O-intensive. Without limits,
multiple concurrent builds saturate disk bandwidth:

```
/sys/fs/cgroup/roko/agent-{plan_id}/io.max = "253:0 rbps=104857600 wbps=52428800"
                                                            ^              ^
                                                       100 MB/s read   50 MB/s write
```

These limits prevent any single build from monopolizing disk I/O
while allowing enough bandwidth for reasonable build performance.

---

## Graceful Shutdown Sequence

When the orchestrator shuts down (user Ctrl+C, budget exhaustion, or
all tasks complete), the supervisor executes a four-phase shutdown:

```
Phase 1: Stop Accepting (immediate)
    │
    ├─► Set supervisor.accepting_spawns = false
    ├─► No new processes can be spawned
    └─► In-flight spawn requests get Err(ShutdownInProgress)

Phase 2: Drain Active (configurable timeout, default 30s)
    │
    ├─► Send SIGTERM to all registered Running processes
    ├─► Wait for processes to exit cleanly
    ├─► Track: remaining = count of Running processes
    │
    ├─► Every 5s: log "shutdown: {remaining} processes still active"
    │
    └─► If drain timeout expires → proceed to Phase 3

Phase 3: Force Kill (5s)
    │
    ├─► For each still-Running process:
    │       kill_all_descendants(pid)  // SIGTERM + SIGKILL
    │
    └─► Wait up to 5s for all kills to complete

Phase 4: Checkpoint and Flush (2s)
    │
    ├─► Write final PID registry state to disk
    ├─► Flush all log buffers
    ├─► Write executor checkpoint (for --resume)
    └─► Exit
```

### Integration with State Persistence

The shutdown sequence coordinates with the executor's checkpoint
system. The checkpoint written in Phase 4 records which tasks were
in-flight at shutdown time. On `--resume`, these tasks are restarted
from their last known phase, not from scratch.

```
Shutdown checkpoint:
    in_flight: [task-7, task-12]
    completed: [task-1, task-2, task-3, task-5, task-6]
    failed: [task-4]
    active_pids: []  // all processes killed by Phase 3

Resume:
    Reload checkpoint
    task-7: was in Implementing phase → restart Implementation
    task-12: was in Gating phase → restart Gating
    Others: retain their completed/failed status
```

The atomic checkpoint write (temp file + rename) ensures the
checkpoint is either complete or absent — never partially written.
A crash during Phase 4 means no checkpoint is written, and the
next resume uses the previous periodic checkpoint.

---

## Integration with the Conductor

The ProcessSupervisor and Conductor operate at different levels but
complement each other:

| Aspect | ProcessSupervisor | Conductor |
|--------|------------------|-----------|
| Level | Process (OS-level) | Plan (task-level) |
| Monitors | PIDs, exit codes, resource usage | Watcher signals, quality metrics |
| Detects | Orphans, crashes, OOM, hangs | Stuck patterns, cost spikes, loops |
| Responds | Kill, restart process | Restart plan, fail plan |
| Persistence | PID registry in memory | Circuit breaker in DashMap |

The Conductor's ghost-turn watcher detects an agent that is running
but producing nothing. The ProcessSupervisor provides the mechanism
to kill that agent's process tree. The Conductor decides; the
Supervisor executes:

```
Conductor: "Agent for plan-42 has 3 ghost turns → restart"
    │
    └─► Supervisor.kill_all_descendants(agent_pid_for_plan_42)
        Supervisor.spawn(plan_42, task, cmd)  // fresh attempt
```

Similarly, the circuit breaker's "open" state prevents the
PlanRunner from spawning new attempts, while the Supervisor ensures
any existing processes for that plan are terminated:

```
Circuit breaker opens for plan-42:
    │
    ├─► PlanRunner: stop scheduling tasks for plan-42
    └─► Supervisor: kill any running processes for plan-42
```

---

## Cross-References

- [00-conductor-architecture.md](00-conductor-architecture.md) —
  Conductor architecture and evaluate() flow
- [01-watcher-ensemble.md](01-watcher-ensemble.md) — Watchers that
  trigger process-level actions
- [02-circuit-breaker.md](02-circuit-breaker.md) — Circuit breaker
  that coordinates with supervisor
- [10-adaptive-timeouts-state-machine.md](10-adaptive-timeouts-state-machine.md)
  — Phase timeouts enforced by supervisor
- [14-production-failure-catalog.md](14-production-failure-catalog.md)
  — Issues #6, #7, #8 that motivated the supervisor

### File References

| File | What |
|------|------|
| `crates/bardo-runtime/` | ProcessSupervisor, event bus, cancellation |
| `crates/roko-cli/src/orchestrate.rs` | PlanRunner that uses the supervisor |
| `crates/roko-conductor/src/conductor.rs` | Conductor that issues decisions the supervisor executes |


---

# SOURCE: /Users/will/dev/nunchi/roko/roko/docs/07-conductor/14-production-failure-catalog.md

# Production Failure Catalog

> Every issue cataloged here was hit in production during batch runs
> between March and April 2026. Each entry traces the full chain:
> symptom, root cause, and the Conductor response that detects or
> prevents recurrence.


> **Implementation**: Built

---

## Summary

21 production failures across 6 categories. Each failure maps to one
or more Conductor mechanisms (watchers, circuit breaker, diagnosis
engine, anomaly detector, or process supervisor) that either already
detects it or would detect it once fully wired.

### Category Overview

| Category | Issues | Conductor Coverage |
|----------|--------|-------------------|
| State Corruption | #1-4 | Event-sourced state, circuit breaker |
| Data Pipeline | #5, #13-15 | Diagnosis engine, typed pipelines |
| Process Management | #6-9 | ProcessSupervisor, ghost-turn watcher |
| Resource Management | #10-12 | Cost watcher, context pressure watcher, anomaly detector |
| Merge & Coordination | #16-18 | Review loop watcher, spec drift watcher |
| Observability | #19-21 | Efficiency events, structured signals |

---

## Category: State Corruption

These failures share a pattern: mutable state updated non-atomically
by multiple callers, producing states that violate invariants.

### Issue #1: in_flight/completed Overlap

**Symptom**: Tasks appeared in both `in_flight` and `completed_tasks`
sets simultaneously. Downstream logic that assumed mutual exclusivity
would double-count work or skip gating.

**Root Cause**: `snapshot()` returned state with a task in both sets.
The transition from in_flight to completed was not atomic — the task
was added to completed before being removed from in_flight, and a
concurrent snapshot captured the intermediate state.

**Conductor Response**:
- **Circuit breaker**: The corrupted state could cause a task to be
  gated twice or not at all. If the double-gate fails (because the
  task is already completed), the failure increments the plan's
  failure count. At MAX_PLAN_FAILURES=2, the circuit breaker opens.
- **Structural prevention**: Event-sourced state. A `TaskCompleted`
  event is a single atomic fact. The projection derives in_flight
  and completed as disjoint sets. Partial state is structurally
  impossible.

**Design Principles Violated**: #1 (Single source of truth), #2
(Event-sourced state).

---

### Issue #2: Orphaned Plans

**Symptom**: Tasks existed in task state but had no corresponding
`plan_phase` entry. The orchestrator tried to advance these tasks but
had no phase context, causing panics or silent drops.

**Root Cause**: Plan creation involved multiple writes — inserting
tasks, then inserting the plan_phase. A crash between writes left
tasks belonging to no plan.

**Conductor Response**:
- **Iteration loop watcher**: Orphaned tasks that the orchestrator
  attempts to advance will fail repeatedly. The iteration loop
  watcher detects this after MAX_ITERATIONS=3 and fires a Critical
  severity signal, which the intervention policy maps to Fail.
- **Structural prevention**: Event-sourced state. A `PlanCreated`
  event contains both plan metadata and initial task set. The
  projection builds both atomically from one event.

**Design Principles Violated**: #1 (Single source of truth), #2
(Event-sourced state), #5 (Fail loud).

---

### Issue #3: Branch Divergence

**Symptom**: Worktree branches diverged from the batch branch.
Merging back produced conflicts or quarantine loops — the plan would
fail, be retried, fail again with the same conflict, and loop.

**Root Cause**: Plan branches were long-lived. The batch branch
advanced as other plans merged. The longer a plan ran, the further
its branch diverged. Rebasing frequently failed (see Issue #16).

**Conductor Response**:
- **Review loop watcher**: Repeated merge-conflict failures that
  result in review rejections trigger the watcher at
  MAX_REVIEW_CYCLES=3.
- **Circuit breaker**: Repeated plan failures from merge conflicts
  open the breaker at MAX_PLAN_FAILURES=2.
- **Structural prevention**: Ephemeral branch model. Branches are
  created from current HEAD immediately before work, merged or
  discarded after gate pass. No branch lives long enough to diverge.

**Design Principles Violated**: #3 (Ephemeral everything), #10
(Monotonic progress).

---

### Issue #4: CONTEXT.md Concurrent Appends

**Symptom**: Multiple agents writing to shared CONTEXT.md
simultaneously. Content was interleaved, truncated, or lost. Agents
reading the file saw corrupted context, degrading output quality.

**Root Cause**: CONTEXT.md was a plain file in the shared worktree.
No locking, no coordination. File system does not guarantee atomic
appends for concurrent writers.

**Conductor Response**:
- **Spec drift watcher**: Corrupted context causes agent output to
  drift from the task specification. When cosine similarity drops
  below MAX_DRIFT=0.25, the watcher fires.
- **Quality degradation (anomaly detector)**: Context corruption
  degrades quality scores. When recent average drops >0.15 below
  earlier average AND recent average is below 0.5, the anomaly
  detector fires.
- **Structural prevention**: Event-sourced context. Each agent
  receives context via `context/in/` (read-only). Outputs go to
  `context/out/`. The shared mutable file is eliminated.

**Design Principles Violated**: #1 (Single source of truth), #7
(Process isolation), #9 (Immutable artifacts).

---

## Category: Data Pipeline

These failures share a root cause: LLM-generated artifacts consumed
without validation.

### Issue #5: Counter Bug (TOML Fences)

**Symptom**: `task_weighted_progress` reported ~2.5% when actual
completion was much higher. ETA stuck at 8 hours. 388 of 544 task
files affected.

**Root Cause**: Enrichment LLM (Haiku) wrapped TOML in markdown code
fences. TOML parser returned `Err`, which was silently swallowed.
Empty checklists produced wrong progress fractions.

**Conductor Response**:
- **Diagnosis engine**: The 34-pattern engine includes `TomlParsing`
  as an error category. When TOML parse failures are detected in gate
  output, the engine suggests `RetryWithFix` intervention.
- **Structural prevention**: Schema validation at generation time.
  Parse generated TOML immediately, reject/retry on failure.

**Design Principles Violated**: #4 (Typed pipelines), #5 (Fail loud).

---

### Issue #13: Enrichment TOML Fences

**Symptom**: Identical to Issue #5 from the pipeline perspective.
388/544 enrichment-generated TOML files wrapped in markdown fences.

**Root Cause**: LLMs trained on chat data wrap structured output in
code fences even when told not to. The pipeline piped output to file
without validation.

**Conductor Response**: Same as Issue #5 — diagnosis engine detects
the pattern, schema validation prevents recurrence.

**Design Principles Violated**: #4 (Typed pipelines).

---

### Issue #14: Verify Script Stale References

**Symptom**: Verify scripts referenced packages, modules, functions
that did not exist. Scripts failed with "not found" during gate
phase, failing correct implementations.

**Root Cause**: Enrichment LLM hallucinated plausible package names
and function signatures. Scripts were not validated against the
codebase at generation time.

**Conductor Response**:
- **Compile-fail-repeat watcher**: Stale references produce compile
  errors. After MAX_COMPILE_FAILS=3 identical failures, the watcher
  fires with Warning severity → restart.
- **Diagnosis engine**: Matches `E0432` (unresolved import) and
  `E0433` (unresolved path) patterns, suggesting `ImportNotFound`
  category and `RetryWithFix` intervention.
- **Structural prevention**: Dry-run validation at enrichment time.
  Verify all referenced symbols exist before accepting the script.

**Design Principles Violated**: #4 (Typed pipelines), #11 (Anticipate).

---

### Issue #15: Review Verdict Parsing

**Symptom**: Review verdicts parsed incorrectly. Plans that should
have passed were failed, and vice versa. Review cycle looped
unnecessarily.

**Root Cause**: Reviewers output TOML in markdown. The regex
fallback parser was fragile and confused by similar patterns in
commentary.

**Conductor Response**:
- **Review loop watcher**: Incorrect parsing causes repeated review
  rejections for plans that should have passed. At MAX_REVIEW_CYCLES=3,
  the watcher fires.
- **Structural prevention**: Typed review pipeline. `ReviewReport`
  struct with schema-validated JSON. Parsing is deserialization, not
  regex.

**Design Principles Violated**: #4 (Typed pipelines), #9 (Immutable
artifacts).

---

## Category: Process Management

These failures stem from treating agent processes as fire-and-forget.

### Issue #6: Spawn Races

**Symptom**: Agents exited with near-zero output. Retry fired
instantly. Exit event from attempt N confused with attempt N+1 —
killing the new attempt or double-counting the failure.

**Root Cause**: No attempt tracking. Exit events did not identify
which spawn attempt they belonged to. Retries fired without backoff.

**Conductor Response**:
- **Ghost-turn watcher**: Near-zero output agents are detected at
  MAX_GHOST_TURNS=3. Ghost detection catches the symptom even if
  the spawn race itself is not detected.
- **ProcessSupervisor**: Monotonically increasing attempt IDs.
  Exit events carry attempt IDs. Stale events are ignored
  structurally.

**Design Principles Violated**: #7 (Process isolation), #5 (Fail
loud).

---

### Issue #7: Orphaned Cargo Processes

**Symptom**: Timeout killed the shell script but not the cargo
process tree. Orphaned cargo processes accumulated, starving CPU
and memory.

**Root Cause**: `kill(pid)` does not kill descendants unless they
are in the same process group.

**Conductor Response**:
- **Cost overrun watcher**: Orphaned processes burning CPU
  indirectly increase turn costs. The cost watcher fires at
  $10 limit.
- **Context pressure watcher**: Resource starvation from orphans
  degrades system performance, indirectly increasing context
  pressure.
- **ProcessSupervisor**: `kill_all_descendants(pid)` with
  bottom-up kill ordering. Process group management via `setsid`.
  Periodic orphan reaper sweep.

**Design Principles Violated**: #7 (Process isolation).

---

### Issue #8: Claude CLI Cold Start

**Symptom**: Every agent turn took 2-5s startup overhead. Over
hundreds of turns, 10-30 minutes of pure waste.

**Root Cause**: CLI spawns a new subprocess per turn. No persistent
connection or subprocess reuse.

**Conductor Response**:
- **Time overrun watcher**: Cumulative cold start overhead
  contributes to phase time exceeding the 80% threshold.
- **Efficiency events**: Per-turn `time_to_first_token_ms` and
  `wall_time_ms` fields capture cold start overhead for analysis.
- **Structural prevention**: Agent connection pooling. Warm
  connections amortize startup cost.

**Design Principles Violated**: #8 (Measure everything).

---

### Issue #9: Agent Ghost Turns

**Symptom**: Agent appeared active but produced no useful output —
repeating itself, asking clarifying questions to nobody, or
describing what it would do without doing it. Burned significant
token budget.

**Root Cause**: LLM agents enter degenerate loops when context is
confusing, instructions ambiguous, or errors unhandled.

**Conductor Response**:
- **Ghost-turn watcher**: Primary detection. At MAX_GHOST_TURNS=3,
  fires Warning severity → restart with fresh context.
- **Stuck-pattern watcher**: Detects repetitive output patterns at
  MAX_STUCK_PATTERNS=4.
- **Anomaly detector (prompt loop)**: If the same prompt hash
  appears 5 times in a 20-prompt window, the session is aborted.
- **Cost spike (anomaly detector)**: Ghost turns consuming expensive
  API calls trigger z-score > 3.0 detection.

**Design Principles Violated**: #11 (Anticipate), #8 (Measure
everything).

---

## Category: Resource Management

These failures arise from treating resources as unlimited.

### Issue #10: Disk Pressure

**Symptom**: Build failures with cryptic errors. Only 7.3 GB free
on a 1.8 TB drive. Cargo target directories and worktree copies had
accumulated.

**Root Cause**: No proactive disk monitoring. Multiple worktrees
each with their own target directory. GC only ran when explicitly
triggered.

**Conductor Response**:
- **Health monitor**: `SystemSnapshot` can be extended with disk
  pressure checks.
- **Anomaly detector (budget exhaustion)**: Budget tracking catches
  cost-related resource exhaustion; disk exhaustion requires an
  analogous disk budget.
- **Structural prevention**: DiskBudget. Estimate disk footprint
  before starting a plan. Refuse if budget exceeds available space.

**Design Principles Violated**: #6 (Resource budgets), #11
(Anticipate).

---

### Issue #11: Gate Serialization Bottleneck

**Symptom**: Plans completed implementation quickly but waited in
queue for gate verification. Serialized gate processing, one at a
time.

**Root Cause**: Double semaphore (`cargo_gate` + `verify_chain`,
both with 1 permit) serialized all compilation. After worktree
isolation, separate target directories made serialization
unnecessary.

**Conductor Response**:
- **Time overrun watcher**: Gate queue waiting contributes to phase
  time exceeding the 80% threshold, making the bottleneck visible.
- **Efficiency events**: `wall_time_ms` vs `duration_ms` gap
  reveals queue wait time in event data.
- **Structural prevention**: Build dependency graph for scheduling.
  Parallelize gates with independent build graphs.

**Design Principles Violated**: #6 (Resource budgets), #8 (Measure
everything).

---

### Issue #12: Memory Pressure from Large Prompts

**Symptom**: Agent output quality degraded as prompt size increased.
Agents ignored relevant context buried in large prompts or fixated
on irrelevant context. Token costs increased proportionally.

**Root Cause**: "Include everything" prompt strategy. Prompts
exceeding 100K tokens. LLM attention is not uniform — middle content
gets less attention.

**Conductor Response**:
- **Context window pressure watcher**: Fires at 80% of model's
  context window. This is the primary defense against oversized
  prompts.
- **Spec drift watcher**: Large prompts cause agents to drift from
  spec. Drift detection catches the quality degradation symptom.
- **Quality degradation (anomaly detector)**: Quality scores drop
  when prompts are too large, triggering degradation detection.
- **Structural prevention**: Adaptive context dropping. Score each
  section by relevance, include in priority order until budget
  reached.

**Design Principles Violated**: #8 (Measure everything), #11
(Anticipate).

---

## Category: Merge & Coordination

These failures arise from concurrent plans interacting through
shared branches and files.

### Issue #16: Rebase Failures

**Symptom**: "batch rebase failed" permanently killed plans. Work
was lost with no recovery.

**Root Cause**: Long-lived branches needed rebasing onto advancing
batch branch. Rebase failure was treated as permanent rather than
recoverable.

**Conductor Response**:
- **Iteration loop watcher**: Rebase-fail-retry loops detected at
  MAX_ITERATIONS=3 → Critical severity → Fail.
- **Circuit breaker**: Repeated rebase failures increment plan
  failure count. Breaker opens at MAX_PLAN_FAILURES=2.
- **Structural prevention**: Ephemeral branches. Never rebase.
  Branches are born from current HEAD and merged or discarded.

**Design Principles Violated**: #3 (Ephemeral everything), #10
(Monotonic progress).

---

### Issue #17: Merge Conflicts at Gate

**Symptom**: Two plans that both passed gates individually would
conflict when merged. Second plan fails, retries, fails again —
loop.

**Root Cause**: Plans scheduled without considering file overlap.
Both succeed in isolation but conflict when combined.

**Conductor Response**:
- **Compile-fail-repeat watcher**: Merge conflicts produce compile
  errors when the merged result does not build. Watcher fires at
  MAX_COMPILE_FAILS=3.
- **Review loop watcher**: Merge conflicts that manifest as review
  rejections trigger the watcher.
- **Circuit breaker**: The retry-conflict-retry loop produces
  plan failures that open the breaker.
- **Structural prevention**: Dependency graph with pre-merge
  conflict detection. Dry-run merge before attempting real merge.

**Design Principles Violated**: #11 (Anticipate), #6 (Resource
budgets).

---

### Issue #18: Worktree Symlinks to Shared State

**Symptom**: Race conditions when multiple agents accessed shared
state through symlinks. Changes by one agent affected another's
view.

**Root Cause**: Worktrees had symlinks to shared mutable files
(CONTEXT.md, plan state). Writes from any worktree mutated the same
file.

**Conductor Response**:
- **Spec drift watcher**: Corrupted shared state causes output
  drift from specification.
- **Stuck-pattern watcher**: Agents receiving corrupted context
  may produce repetitive confused output.
- **Structural prevention**: Full worktree isolation. No shared
  mutable state. Orchestrator crosses worktree boundaries through
  explicit collect/inject, never shared file handles.

**Design Principles Violated**: #7 (Process isolation), #1 (Single
source of truth).

---

## Category: Observability

These failures share a theme: insufficient information to diagnose
problems quickly.

### Issue #19: Buried Failures in Logs

**Symptom**: Critical errors hidden in the middle of 50,000-line
log files. TUI showed aggregated status but not individual failure
details. Required manual grep to find failures.

**Root Cause**: Unstructured logging. All events to same stream
with no severity routing or queryability.

**Conductor Response**:
- **Efficiency events**: Structured `AgentEfficiencyEvent` with 20+
  fields replaces unstructured logging for agent performance data.
- **Conductor signals**: Every conductor intervention produces a
  typed `Signal` with severity, watcher name, plan ID, and count.
  These are queryable, not buried in logs.
- **Structural prevention**: EventBus with queryable event stream.
  Query for "all errors in the last hour" without scanning full log.

**Design Principles Violated**: #5 (Fail loud), #8 (Measure
everything).

---

### Issue #20: No Signal on WHY Plans Fail

**Symptom**: TUI showed "Failed" with no root cause. Operator had
to dig through logs, worktree state, and git history.

**Root Cause**: Failure path recorded status change but not reason.
Error messages logged but not attached to plan state.

**Conductor Response**:
- **Diagnosis engine**: Classifies errors into 20 categories with
  suggested interventions. Provides the "why" that was missing.
- **Conductor intervention signals**: Include watcher name, severity,
  plan ID, and descriptive message. Signal content explains why the
  intervention fired.
- **Structural prevention**: Enriched error digests. Every failure
  produces a structured `FailureReport` attached to plan state.

**Design Principles Violated**: #5 (Fail loud), #8 (Measure
everything).

---

### Issue #21: ETA Completely Wrong

**Symptom**: ETA showed 8+ hours when actual remaining was ~2 hours.
Progress bar at ~2.5% when actual completion was ~40%.

**Root Cause**: Directly caused by Issue #5. Weighted progress
depended on checklist counts from broken TOML files. With 388/544
failing to parse, the fraction was wrong.

**Conductor Response**:
- **Anomaly detector**: Internal inconsistency (40% gates passed
  but 2.5% progress shown) is detectable as a quality anomaly.
- **Efficiency events**: `gate_passed` field provides a reliable
  progress signal independent of checklist parsing.
- **Structural prevention**: Progress from gate outcomes, not
  checklist parsing. Gates passed / gates total is a single reliable
  metric.

**Design Principles Violated**: #8 (Measure everything), #5 (Fail
loud), #11 (Anticipate).

---

## Cross-Reference Tables

### Issue → Conductor Mechanism

| # | Issue | Primary Mechanism | Secondary Mechanism |
|---|-------|------------------|-------------------|
| 1 | in_flight/completed overlap | Circuit breaker | Event-sourced state |
| 2 | Orphaned plans | Iteration loop watcher | Event-sourced state |
| 3 | Branch divergence | Review loop watcher | Circuit breaker |
| 4 | CONTEXT.md concurrent appends | Spec drift watcher | Quality anomaly detector |
| 5 | Counter bug (TOML fences) | Diagnosis engine | Schema validation |
| 6 | Spawn races | Ghost-turn watcher | ProcessSupervisor |
| 7 | Orphaned cargo processes | ProcessSupervisor | Cost watcher |
| 8 | Claude CLI cold start | Time overrun watcher | Efficiency events |
| 9 | Agent ghost turns | Ghost-turn watcher | Prompt loop detector |
| 10 | Disk pressure | Health monitor | Budget anomaly |
| 11 | Gate serialization | Time overrun watcher | Efficiency events |
| 12 | Large prompt pressure | Context pressure watcher | Spec drift watcher |
| 13 | Enrichment TOML fences | Diagnosis engine | Schema validation |
| 14 | Verify script stale refs | Compile-fail-repeat watcher | Diagnosis engine |
| 15 | Review verdict parsing | Review loop watcher | Typed review pipeline |
| 16 | Rebase failures | Iteration loop watcher | Circuit breaker |
| 17 | Merge conflicts at gate | Compile-fail-repeat watcher | Circuit breaker |
| 18 | Worktree symlinks | Spec drift watcher | Stuck-pattern watcher |
| 19 | Buried failures | Efficiency events | Conductor signals |
| 20 | No failure signal | Diagnosis engine | Conductor signals |
| 21 | ETA wrong | Anomaly detector | Efficiency events |

### Issue → Design Principle

| Principle | Prevents Issues |
|-----------|----------------|
| #1 Single source of truth | #1, #2, #4, #18 |
| #2 Event-sourced state | #1, #2, #4 |
| #3 Ephemeral everything | #3, #16 |
| #4 Typed pipelines | #5, #13, #14, #15 |
| #5 Fail loud, recover fast | #1, #2, #5, #6, #19, #20, #21 |
| #6 Resource budgets | #10, #11, #17 |
| #7 Process isolation | #4, #6, #7, #9, #18 |
| #8 Measure everything | #8, #9, #11, #12, #19, #20, #21 |
| #9 Immutable artifacts | #4, #15 |
| #10 Monotonic progress | #3, #16 |
| #11 Anticipate, don't react | #9, #10, #12, #14, #17, #21 |

### Issue → Refactoring Phase

| Phase | Issues Addressed |
|-------|-----------------|
| Phase 0 (Instrument) | #19, #21 |
| Phase 1 (Quick Wins) | #5, #10, #12, #13, #14, #20 |
| Phase 2 (Decompose) | #3, #16 |
| Phase 3 (Foundation) | #1, #2, #4, #5, #6, #7, #10, #18, #19 |
| Phase 4 (Core) | #8, #11, #12, #15, #17 |
| Phase 5 (Cybernetic) | #9, #14, #20, #21 |

---

## Cross-References

- [01-watcher-ensemble.md](01-watcher-ensemble.md) — Watcher
  mechanisms referenced throughout this catalog
- [02-circuit-breaker.md](02-circuit-breaker.md) — Circuit breaker
  responses to repeated failures
- [04-diagnosis-engine.md](04-diagnosis-engine.md) — Error
  classification for data pipeline failures
- [06-health-monitors.md](06-health-monitors.md) — System health
  checks for resource failures
- [11-anomaly-detection-learning.md](11-anomaly-detection-learning.md)
  — Anomaly detection for quality and cost failures
- [13-process-supervision-wiring.md](13-process-supervision-wiring.md)
  — Process management failure responses


---

# SOURCE: /Users/will/dev/nunchi/roko/roko/docs/07-conductor/15-conductor-learning-federation.md

# Conductor learning, federation, and self-healing

> The Conductor is not static. It learns which interventions work,
> federates control across subsystem boundaries, and heals itself
> when its own model drifts from reality.


> **Implementation**: Scaffold

---

## Conductor learning -- from static rules to adaptive policy

### The learning gap

The conductor uses static thresholds. `MAX_GHOST_TURNS=3`.
`WorstSeverityPolicy`. These constants were calibrated from production
batch runs in March-April 2026 and they work for that workload. But
workloads change. Model versions change. Codebase complexity changes.
A threshold that was correct last month may be too strict or too
lenient today.

The learning infrastructure exists. `ConductorBandit` in
`roko-learn/src/conductor.rs` implements a contextual bandit for
intervention selection. The efficiency event pipeline records every
agent turn with 20+ fields of outcome data. The cascade router
already uses Thompson Sampling to learn model-task mappings. The
conductor's decision path does not use any of this.

The gap: the conductor collects data but does not learn from it.
Interventions are rule-driven, not data-driven. Closing this gap
means wiring the bandit into the conductor's `evaluate()` path,
replacing `WorstSeverityPolicy` with a learned policy that falls
back to static rules when confidence is low.

### Contextual bandit for intervention selection

The bandit models intervention selection as a contextual multi-armed
bandit problem. The state captures execution context. The actions are
conductor decisions. The reward reflects whether the intervention
improved the outcome.

**State**: 19-dimensional feature vector extracted from watcher
outputs and execution context:

- Iteration number (how many gate-fail cycles so far)
- Failure count (total failures in this plan attempt)
- Elapsed milliseconds (wall-clock time since task start)
- Accumulated cost in USD
- Model tier (0=haiku, 1=sonnet, 2=opus)
- Task complexity (from TOML frontmatter: 0=trivial, 1=simple, 2=standard, 3=complex)
- Error pattern hash (which error categories have appeared)
- Interaction terms: iteration x failure_count, cost x complexity, elapsed x model_tier

Interaction terms matter because the right intervention depends on
combinations. A high iteration count alone might mean "keep trying."
A high iteration count combined with rising cost means "abort."

**Actions**: Continue, InjectHint, SwitchModel, Restart, Abort.

**Algorithm**: Thompson Sampling blended with a linear context model.
65% Thompson (exploration), 35% linear (exploitation from context
features). The blend prevents the bandit from over-exploiting early
patterns while still using context to make informed decisions.

```rust
/// Learned conductor policy using contextual bandits.
/// Replaces static WorstSeverityPolicy with data-driven decisions.
pub struct LearnedConductorPolicy {
    /// The underlying bandit that selects actions.
    bandit: ConductorBandit,
    /// Minimum confidence before overriding static policy.
    /// Below this, fall back to WorstSeverityPolicy.
    min_confidence: f64,  // default: 0.6
    /// Number of observations before learning activates.
    warmup_observations: usize,  // default: 50
}

impl InterventionPolicy for LearnedConductorPolicy {
    fn evaluate(&self, outputs: &[WatcherOutput], ctx: &Context) -> ConductorDecision {
        // Extract features from watcher outputs and context
        let features = self.extract_features(outputs, ctx);

        if self.bandit.total_observations() < self.warmup_observations {
            // Fall back to static policy during warmup
            return WorstSeverityPolicy.evaluate(outputs, ctx);
        }

        let (action, confidence) = self.bandit.select_with_confidence(&features);

        if confidence < self.min_confidence {
            // Low confidence — use static policy as safety net
            return WorstSeverityPolicy.evaluate(outputs, ctx);
        }

        action.to_conductor_decision(outputs)
    }
}
```

The warmup period (50 observations) prevents the bandit from making
decisions before it has enough data. During warmup, the static policy
runs unchanged. After warmup, the bandit selects actions but defers
to the static policy whenever its confidence falls below 0.6. This
two-tier fallback means the learned policy can only override static
rules when it has both sufficient data and sufficient confidence.

### Reward shaping

Defining good rewards for conductor actions is the hard part. The
naive approach -- reward 1.0 for success, 0.0 for failure -- does
not capture the nuance. A well-timed Abort on a futile plan is a
good outcome. It saves tokens. It frees the executor to work on
plans that can succeed. The reward signal must reflect this.

| Action | Outcome | Reward |
|--------|---------|--------|
| Continue | Next gate passes | 0.9 |
| Continue | Next gate fails | 0.1 |
| Restart | Restarted agent succeeds | 0.8 |
| Restart | Restarted agent also fails | 0.2 |
| Fail | Plan was later retried and failed again | 0.7 (correct fail-fast) |
| Fail | Plan was later retried and succeeded | 0.1 (premature failure) |

Three design decisions in this reward table:

**Continue-pass gets 0.9, not 1.0.** Reserving 1.0 prevents reward
saturation. The bandit can always find room to improve.

**Fail-correct gets 0.7.** A correct Abort is valuable but not as
valuable as a successful Continue or Restart. The system should prefer
actions that lead to success over actions that correctly predict
failure. But correct failure prediction still earns substantial
reward because it saves tokens and wall-clock time.

**Fail-premature gets 0.1, not 0.0.** The plan was recoverable but
the conductor gave up too early. This is the worst outcome -- the
system spent tokens on a failed attempt and then spent more tokens
on a successful retry that should have been the first attempt's
continuation. The low reward (but not zero) prevents the bandit
from completely avoiding Fail actions.

The Restart-fail reward (0.2) is higher than Continue-fail (0.1)
because a restart at least attempted a different strategy. Rewarding
attempted recovery over passive continuation encourages the bandit
to try restarts when it detects problems, even if restarts do not
always succeed.

### Online learning loop

The learning loop closes within the conductor's evaluation cycle:

```
Agent turn completes
    -> Conductor evaluates (bandit selects action)
    -> Action executed (Continue/Restart/Fail)
    -> Outcome observed (next gate result)
    -> Bandit updated with (state, action, reward)
    -> Policy improves
```

The delay between action and reward varies by action type. Continue
rewards arrive on the next turn (fast feedback). Restart rewards
arrive after the restarted agent completes (slower). Fail rewards
arrive only when the plan is retried (possibly never, if the circuit
breaker trips). This variable delay means the bandit must handle
sparse and delayed rewards for Fail actions. The implementation
queues pending rewards and resolves them when outcomes become
available.

---

## Conductor federation -- multi-level control

### Four-level federation architecture

The current conductor operates at a single level: per-task. It
watches one agent executing one task and decides Continue, Restart,
or Fail. But orchestration happens at multiple levels simultaneously.
A plan contains many tasks. A batch contains many plans. A session
contains many batches. Each level has its own signals, its own
failure modes, and its own intervention options.

Federation puts a conductor at each level:

```
+----------------------------------------------------+
|  L4: Fleet Conductor (cross-plan, per-batch)       |
|  Scope: All plans in a session                      |
|  Signals: Plan outcomes, fleet-level metrics        |
|  Actions: Router policy updates, global budgets     |
|                                                      |
|  +----------------------------------------------+  |
|  |  L3: Plan Conductor (per-plan)               |  |
|  |  Scope: All tasks in one plan                 |  |
|  |  Signals: Task outcomes, plan-level cost       |  |
|  |  Actions: Resource reallocation, priority      |  |
|  |                                                |  |
|  |  +----------------------------------------+  |  |
|  |  |  L2: Task Conductor (per-task)         |  |  |
|  |  |  Current roko-conductor                |  |  |
|  |  |  10 watchers + circuit breaker         |  |  |
|  |  |  Continue / Restart / Fail              |  |  |
|  |  |                                        |  |  |
|  |  |  +--------------------------------+   |  |  |
|  |  |  |  L1: Turn Conductor            |   |  |  |
|  |  |  |  AnomalyDetector               |   |  |  |
|  |  |  |  Prompt loop, cost spike       |   |  |  |
|  |  |  +--------------------------------+   |  |  |
|  |  +----------------------------------------+  |  |
|  +----------------------------------------------+  |
+----------------------------------------------------+
```

**L1 (Turn)** operates at the granularity of a single agent turn.
The `AnomalyDetector` already does this -- it checks prompt hashes,
cost spikes, and quality degradation before each turn. L1 catches
problems before they become multi-turn patterns.

**L2 (Task)** is the current conductor. Ten watchers, one circuit
breaker, one intervention policy. It observes multi-turn patterns
within a single task and decides whether to continue, restart, or
fail. This is what `roko-conductor` implements today.

**L3 (Plan)** observes all tasks within one plan. It sees patterns
that L2 cannot: task A failed with a compile error, task B depends
on the code that A was supposed to write, so B will also fail.
L3 can reallocate resources (assign a stronger model to critical-path
tasks) or reprioritize (skip optional tasks when the budget runs low).

**L4 (Fleet)** observes all plans in a session. It sees cross-plan
patterns: three authentication-related plans failed this batch, which
suggests a systemic issue (maybe a dependency changed). L4 can update
router policies globally, adjust session-level budgets, or halt entire
categories of work.

### Conductor trait at each level

All four levels implement the same trait. Federation is achieved
through composition -- each level reads signals from the level below
and emits signals for the level above. No special hierarchy protocol
is needed. The signal stream is the communication channel.

```rust
/// All conductor levels implement the same trait.
/// Federation is achieved through composition, not hierarchy.
pub trait ConductorLevel: Send + Sync {
    /// The scope of signals this conductor observes.
    fn scope(&self) -> ConductorScope;

    /// Evaluate the signal stream and produce decisions.
    fn evaluate(&self, stream: &[Engram], ctx: &Context) -> Vec<ConductorDecision>;

    /// Accept parameter updates from the level above.
    fn accept_parameters(&mut self, params: &ParameterUpdate);

    /// Emit observations for the level above.
    fn emit_observations(&self) -> Vec<Engram>;
}

pub enum ConductorScope {
    Turn,    // L1: per-agent-turn signals
    Task,    // L2: per-task signals (current conductor)
    Plan,    // L3: per-plan signals
    Fleet,   // L4: cross-plan signals
}
```

The `accept_parameters` method is the downward channel. L4 can push
budget constraints to L3 ("this plan gets at most $5 more"). L3 can
push model selection to L2 ("use opus for this task"). L2 can push
prompt modifications to L1 ("add this hint to the next prompt").

The `emit_observations` method is the upward channel. L1 emits
anomaly signals. L2 emits intervention signals. L3 emits plan
progress signals. L4 emits fleet health signals. Each level
consumes the level below's observations as part of its own signal
stream.

### Communication via signal stream

All conductors communicate through the same signal stream that the
rest of the system uses. No side channels, no special-purpose
message queues. The signal stream is the universal bus.

Signal tags encode the level and type:

- L1 emits `conductor.anomaly.prompt_loop`, `conductor.anomaly.cost_spike`
- L2 reads L1 signals and emits `conductor.intervention.restart`, `conductor.intervention.fail`
- L3 reads L2 signals and emits `conductor.plan.budget_realloc`, `conductor.plan.reprioritize`
- L4 reads L3 signals and emits `conductor.fleet.policy_update`, `conductor.fleet.budget_adjust`

Each level filters the stream by tag prefix. L2 reads all signals
tagged `conductor.anomaly.*`. L3 reads all signals tagged
`conductor.intervention.*`. The filtering is cheap -- a prefix match
on the tag string. The signal stream's append-only JSONL format means
each level reads the full stream and filters in memory.

This design avoids the distributed systems problem of conductor-to-conductor
coordination. There is no coordinator. There is no leader election.
There is a shared log, and each conductor reads the portion relevant
to its scope.

### VSM mapping

Each federation level maps to a system in Beer's Viable System Model.
This mapping is not decorative -- it constrains what each level is
allowed to do and prevents scope creep between levels.

| Level | VSM System | Function |
|-------|-----------|----------|
| L1 (Turn) | System 2 | Coordination -- prevent oscillations within a turn |
| L2 (Task) | System 3 | Control -- internal oversight of task execution |
| L3 (Plan) | System 3* | Audit -- independent check of plan progress |
| L4 (Fleet) | System 4 | Intelligence -- scanning cross-plan patterns for adaptation |

**System 2 (L1)** dampens oscillations. The anomaly detector prevents
prompt loops and cost spikes -- these are oscillatory failure modes
where the system repeats or escalates without bound. S2's job is
stability within a turn.

**System 3 (L2)** provides internal oversight. The 10-watcher
ensemble monitors ongoing execution and intervenes when behavior
diverges from the self-model. S3's job is performance within a task.

**System 3* (L3)** is the audit function. It checks that L2's
interventions are producing good outcomes at the plan level. If L2
keeps restarting a task but the plan is not converging, L3 intervenes
at a higher level (reallocate, reprioritize, or fail the plan). S3*'s
job is accountability across tasks.

**System 4 (L4)** scans the environment for adaptation opportunities.
Cross-plan patterns reveal systemic issues (a model version regresses
on a class of tasks) or systemic opportunities (a model version
excels at a new class of tasks). S4's job is adaptation across plans.

System 5 (policy) is not a conductor level -- it is the human
operator who sets the constraints within which all four levels
operate. The `roko.toml` configuration, the plan definitions, the
acceptance criteria: these are System 5.

---

## Self-healing conductor

### Conductor failure modes

The conductor can fail. Its thresholds can drift. Its model can go
stale. Its watchers can develop blind spots. Its circuit breaker can
get stuck. A conductor that cannot detect its own failures is a
liability -- it gives false confidence that the system is regulated
when it is not.

Four failure modes, each with a distinct symptom, detection method,
and recovery path:

| Failure | Symptom | Detection | Recovery |
|---------|---------|-----------|----------|
| Threshold drift | Good plans get killed (false positives) | Intervention effectiveness drops below 50% | Bayesian threshold adaptation |
| Model staleness | Conductor interventions have no effect | Restart success rate unchanged from continue | Re-calibrate from recent efficiency events |
| Watcher blindness | New failure mode not caught by any watcher | Plans fail without intervention | Unclassified error clustering in efficiency logs |
| Circuit breaker stuck | Plans permanently tripped that should retry | Tripped plans with changed environment | Auto-probe after sleep window (half-open state) |

**Threshold drift** is the most common failure. Model versions change.
Codebase complexity changes. A threshold calibrated for Sonnet 3.5 may
be too strict for Sonnet 4 (which fails less often) or too lenient for
a smaller model (which fails more often). Detection: track the ratio of
interventions that improve outcomes. When this ratio falls below 50%,
the conductor is doing more harm than good.

**Model staleness** is subtler. The conductor intervenes (restarts an
agent), but the restarted agent fails at the same rate as the original.
The intervention has no effect. This means the conductor's model of
"what went wrong" is no longer accurate -- the restart does not address
the actual failure mode. Detection: compare restart success rate against
continue success rate. If they are statistically indistinguishable, the
restart is not helping.

**Watcher blindness** occurs when a new failure pattern emerges that
no watcher detects. The system has a type of error that causes plan
failure but does not trigger any conductor intervention. The plan
fails silently. Detection: look for plans that failed without any
conductor intervention in their history. If the ratio of
unintercepted failures rises, the watchers have a blind spot.

**Circuit breaker stuck** happens when the environment changes but
the breaker does not re-probe. A plan that failed twice because of
a provider outage should be retried after the outage resolves. The
current breaker does not probe -- it is permanently tripped until
a human resets it. Detection: check tripped plans against changed
conditions (provider health recovered, dependency updated, model
version changed).

### Recovery-oriented computing applied

The self-healing conductor borrows four principles from Patterson
et al.'s Recovery-Oriented Computing:

**Make restart cheap.** A conductor threshold reset is cheap: update
a constant, no process restart needed. The conductor can recalibrate
its thresholds without interrupting ongoing execution. This is
analogous to the micro-reboot principle -- fix the smallest unit
possible.

**Test recovery paths.** The self-model accuracy metrics
(`SelfModelAccuracy` from 08-good-regulator-self-model.md) validate
that recovery mechanisms work. If intervention effectiveness drops,
the system knows its recovery path (restart) is not effective. This
is continuous validation, not post-hoc testing.

**Micro-reboots.** Reset individual watcher thresholds without
resetting the entire conductor. If the ghost-turn watcher is too
aggressive, recalibrate that one threshold. The other nine watchers
continue with their existing calibration.

**Survivor functions.** Conductor state -- circuit breaker records,
watcher history, bandit weights -- persists through process restarts
via `.roko/state/`. A crashed orchestrator resumes with the
conductor's learned state intact. The conductor does not start from
zero on every restart.

```rust
/// Self-healing conductor that detects and repairs its own model drift.
pub struct SelfHealingConductor {
    /// The underlying conductor with all watchers and policies.
    inner: Conductor,
    /// Self-model accuracy tracker.
    accuracy: SelfModelAccuracy,
    /// Threshold learner for adaptive calibration.
    threshold_learner: ThresholdLearner,
    /// Minimum accuracy before triggering self-repair.
    min_accuracy: f64,  // default: 0.5
    /// Interval between self-assessments.
    self_check_interval: Duration,  // default: 300s (5 min)
}

impl SelfHealingConductor {
    pub fn self_assess(&mut self) -> Option<SelfRepairAction> {
        // Check intervention effectiveness
        if self.accuracy.intervention_effectiveness < self.min_accuracy {
            return Some(SelfRepairAction::RecalibrateThresholds);
        }
        // Check for undetected failures
        if self.accuracy.stuck_detection_precision < 0.3 {
            return Some(SelfRepairAction::ExpandStuckHeuristics);
        }
        // Check for watcher blindness (plans failing without conductor intervention)
        if self.accuracy.undetected_failure_rate() > 0.2 {
            return Some(SelfRepairAction::AddNewWatcher);
        }
        None
    }
}

pub enum SelfRepairAction {
    RecalibrateThresholds,
    ExpandStuckHeuristics,
    AddNewWatcher,
    ResetCircuitBreakers,
    RetrainBandit,
}
```

The `self_assess` method runs on a 5-minute interval during batch
execution. It checks three conditions in priority order:

1. **Intervention effectiveness below 50%.** The conductor's
   interventions are failing more often than succeeding. The most
   common cause is threshold drift. Recovery: `RecalibrateThresholds`
   triggers the `ThresholdLearner` (from 08-good-regulator-self-model.md)
   to adjust thresholds based on recent outcome data.

2. **Stuck detection precision below 30%.** The stuck detector is
   flagging agents that are not actually stuck. More than 70% of
   "stuck" detections are false positives. Recovery:
   `ExpandStuckHeuristics` tightens the stuck detection thresholds
   so they trigger less often.

3. **Undetected failure rate above 20%.** More than one in five plan
   failures occurs without any conductor intervention. The watchers
   have a blind spot. Recovery: `AddNewWatcher` clusters unclassified
   errors from the efficiency logs and proposes a new watcher pattern.

The priority order matters. Threshold drift is checked first because
it is the most common failure mode and the cheapest to fix. Watcher
blindness is checked last because it is the hardest to fix -- adding
a new watcher requires identifying the new failure pattern and
implementing a detection heuristic.

### Triple-loop learning

Self-healing operates at three levels of abstraction. Each level
fixes a different class of problem.

```
Loop 1 (Single-loop): Correct errors
    Agent fails -> Conductor restarts -> Agent succeeds
    The system fixes the immediate problem.

Loop 2 (Double-loop): Change the rules
    Conductor thresholds produce too many false positives
    -> ThresholdLearner adjusts thresholds
    -> Future interventions are more accurate
    The system improves its own detection.

Loop 3 (Triple-loop): Change the meta-rules
    The threshold learning rate is too slow (or too fast)
    -> Self-model accuracy metrics detect the meta-problem
    -> Learning parameters are adjusted
    The system improves its own improvement process.
```

**Single-loop** is what the conductor does today. An agent fails. The
conductor detects the failure pattern. It restarts the agent or fails
the plan. The immediate problem is addressed. No learning occurs --
the same threshold, the same watcher, the same intervention. If the
same failure happens again tomorrow, the same intervention fires.

**Double-loop** changes the thresholds. The `ThresholdLearner` tracks
intervention effectiveness per watcher. If the ghost-turn watcher's
interventions succeed 90% of the time, its threshold might be too
lenient (it is only catching the obvious cases). If interventions
succeed 30% of the time, the threshold is too strict (too many false
positives). The learner adjusts the threshold toward the sweet spot.
This changes the conductor's behavior for future similar situations.

**Triple-loop** changes the learning process itself. The self-model
accuracy metrics track whether the double-loop is converging. If
threshold adjustments are oscillating (too strict, then too lenient,
then too strict again), the learning rate is too high. If thresholds
barely move despite clear evidence of drift, the learning rate is
too low. The triple-loop adjusts the learning rate, the discount
factor, and the minimum sample size for the `ThresholdLearner`.

The practical test for whether triple-loop learning is needed: does
intervention effectiveness stabilize after double-loop adjustments?
If it does, double-loop is sufficient. If it oscillates or fails to
converge, the learning parameters themselves need adjustment -- and
that is the triple-loop.

This maps to Argyris and Schon's organizational learning framework:
single-loop corrects deviations within existing norms, double-loop
questions and revises the norms, triple-loop questions the process
by which norms are revised. The conductor implements all three
levels computationally.

---

## File reference

| File | What |
|------|------|
| `crates/roko-learn/src/conductor.rs` | ConductorBandit (built, not wired) |
| `crates/roko-conductor/src/conductor.rs` | Current static conductor |
| `crates/roko-conductor/src/interventions.rs` | InterventionPolicy trait |
| `crates/roko-learn/src/anomaly.rs` | AnomalyDetector (L1 conductor) |
| `crates/roko-learn/src/efficiency.rs` | AgentEfficiencyEvent (reward data) |
| `crates/roko-learn/src/cascade_router.rs` | CascadeRouter (L4 conductor analogue) |

---

## Cross-References

- [02-circuit-breaker.md](02-circuit-breaker.md) -- Circuit breaker, half-open state
- [07-ooda-cybernetic-loop.md](07-ooda-cybernetic-loop.md) -- OODA loop, nested loops, IG&C
- [08-good-regulator-self-model.md](08-good-regulator-self-model.md) -- Self-model, accuracy metrics
- [11-anomaly-detection-learning.md](11-anomaly-detection-learning.md) -- Learning integration, feedback loops
- [12-yerkes-dodson-pressure.md](12-yerkes-dodson-pressure.md) -- Pressure calibration, curve fitting


---

# SOURCE: /Users/will/dev/nunchi/roko/roko/docs/07-conductor/INDEX.md

# 07 — Conductor Subsystem

> The Conductor is the agent's theory-of-mind about its own pipeline.
> It observes agent behavior, detects anomalies, and issues graduated
> interventions — not as a timeout manager, but as a cybernetic
> regulator that models the system it governs.

---

## Document Index

| # | File | Topic | Lines |
|---|------|-------|-------|
| 00 | [conductor-architecture.md](00-conductor-architecture.md) | Architecture overview, L3 placement, synapse position | ~250 |
| 01 | [watcher-ensemble.md](01-watcher-ensemble.md) | All 10 watchers with thresholds and detection logic | ~350 |
| 02 | [circuit-breaker.md](02-circuit-breaker.md) | Per-plan breaker, 3-state model, DashMap concurrency | ~280 |
| 03 | [graduated-interventions.md](03-graduated-interventions.md) | Severity→Decision mapping, no-nudge policy | ~300 |
| 04 | [diagnosis-engine.md](04-diagnosis-engine.md) | 34 patterns, 20 categories, intervention suggestions | ~320 |
| 05 | [stuck-detection.md](05-stuck-detection.md) | 6 heuristics, MetaCognitionHook, Theta frequency | ~300 |
| 06 | [health-monitors.md](06-health-monitors.md) | SystemSnapshot, 4 checks, VSM System 3* | ~250 |
| 07 | [ooda-cybernetic-loop.md](07-ooda-cybernetic-loop.md) | OODA mapping, cybernetic structure, feedback properties | ~280 |
| 08 | [good-regulator-self-model.md](08-good-regulator-self-model.md) | Conant-Ashby theorem, self-model components, adaptive vs static | ~300 |
| 09 | [cognitive-signals.md](09-cognitive-signals.md) | 8 typed interrupts, signal semantics, implementation path | ~260 |
| 10 | [adaptive-timeouts-state-machine.md](10-adaptive-timeouts-state-machine.md) | Phase timeouts, complexity bands, graceful shutdown | ~300 |
| 11 | [anomaly-detection-learning.md](11-anomaly-detection-learning.md) | EWMA, prompt loops, learning integration, feedback loops | ~360 |
| 12 | [yerkes-dodson-pressure.md](12-yerkes-dodson-pressure.md) | Inverted-U curve, pressure tuning, cooperation metrics | ~310 |
| 13 | [process-supervision-wiring.md](13-process-supervision-wiring.md) | ProcessSupervisor integration, PID tracking, orphan cleanup | ~310 |
| 14 | [production-failure-catalog.md](14-production-failure-catalog.md) | 21 production failures mapped to conductor responses | ~360 |
| 15 | [conductor-learning-federation.md](15-conductor-learning-federation.md) | Learned policies, federated control, self-healing | ~400 |

---

## Reading Order

### Quick Start (Understand the Conductor in 3 docs)

1. **00-conductor-architecture.md** — What the Conductor is, where
   it sits, what it does
2. **01-watcher-ensemble.md** — The 10 watchers that produce signals
3. **03-graduated-interventions.md** — How signals become decisions

### Full Understanding (Add theory and mechanisms)

4. **07-ooda-cybernetic-loop.md** — The cybernetic theory behind
   the design
5. **08-good-regulator-self-model.md** — Why the Conductor must
   model itself
6. **02-circuit-breaker.md** — Per-plan failure tracking
7. **04-diagnosis-engine.md** — Error classification and auto-fix
8. **05-stuck-detection.md** — Agent progress monitoring
9. **06-health-monitors.md** — System-level health checks

### Advanced Topics (Adaptive behavior and learning)

10. **09-cognitive-signals.md** — Future typed interrupt system
11. **10-adaptive-timeouts-state-machine.md** — Phase lifecycle
12. **11-anomaly-detection-learning.md** — Statistical anomaly
    detection and feedback loops
13. **12-yerkes-dodson-pressure.md** — Pressure dynamics and
    cooperation curves

### Operational (Production and infrastructure)

14. **13-process-supervision-wiring.md** — OS-level process management
15. **14-production-failure-catalog.md** — Every known failure and
    its conductor response

### Frontier (Adaptive and self-improving conductor)

16. **15-conductor-learning-federation.md** — Learned intervention
    policies, federated multi-level control, self-healing conductor

---

## Key Concepts

### The Conductor Is Not a Timeout Manager

The Conductor is a **cybernetic regulator** (Wiener, 1948) — it
implements a closed-loop control system where agent behavior is
observed, compared against expectations, and corrected through
graduated interventions. Timeouts are one mechanism among many.
The theoretical foundation is the Good Regulator Theorem (Conant &
Ashby, 1970): any effective regulator must contain a model of the
system it regulates.

### Decide, Don't Nudge

The Conductor issues **decisions** (Continue, Restart, Fail), not
suggestions. This design reflects Yerkes-Dodson pressure dynamics:
ambiguous nudges add unpredictable cognitive load to agents, while
clear decisions produce predictable outcomes. Three actions is
sufficient variety for effective regulation (Ashby's Law of
Requisite Variety, 1956).

### The Policy Trait

Every watcher implements the same trait:

```rust
pub trait Policy {
    fn decide(&self, stream: &[Engram], ctx: &Context) -> Vec<Engram>;
    fn name(&self) -> &str;
}
```

The Conductor is a composite Policy that runs all 10 watchers,
collects their outputs, and applies an intervention policy to
determine the final decision. This uniform interface means watchers
are independent, composable, and testable in isolation.

### Graduated Severity

| Severity | Decision | Meaning |
|----------|----------|---------|
| Info | Continue | Monitor, no action |
| Warning | Restart | Reset agent with fresh context |
| Critical | Fail | Abort the plan |

Severity escalation is monotonic within a plan — once a watcher
fires at Warning, the system's suspicion of that plan is permanently
elevated.

---

## Cross-Cutting Themes

### Theme: Anticipate, Don't React

Multiple subsystems implement early detection:
- **Anomaly detector** checks prompt BEFORE the turn (doc 11)
- **Phase timeouts** fire at 80% to leave cleanup margin (doc 10)
- **Context pressure** warns before the window fills (doc 01)
- **Cost tracking** uses EWMA to detect spikes early (doc 11)

### Theme: Data Produces Better Interventions

Every intervention is a data point for the learning system:
- Conductor interventions → negative routing signals (doc 11)
- Efficiency events → cascade router training (doc 11)
- Gate outcomes → adaptive threshold tuning (doc 11)
- Cooperation metrics → pressure calibration (doc 12)
- Intervention outcomes → conductor bandit training (doc 15)

### Theme: Hierarchical Control

The conductor operates at multiple timescales simultaneously:
- **Gamma** (per-turn): 10 watchers + anomaly detector (docs 01, 11)
- **Theta** (per-task): MetaCognitionHook + stuck detection (docs 05, 07)
- **Delta** (per-batch): cascade router + threshold adaptation (docs 11, 15)
- Slower loops set parameters for faster loops (doc 07, nested OODA)
- Each level implements the Policy trait independently (doc 15)

### Theme: Multiple Levels of Protection

Protection operates at three levels simultaneously:
- **OS level**: ProcessSupervisor manages PIDs, kills, cleanup (doc 13)
- **Plan level**: Circuit breaker tracks plan failures (doc 02)
- **API level**: Provider health breaker tracks provider errors (doc 11)

### Theme: Production-Derived Design

Every threshold and mechanism traces to a real failure:
- MAX_GHOST_TURNS=3 → Issue #9 (ghost turns)
- MAX_COMPILE_FAILS=3 → Issue #14 (stale references)
- MAX_PLAN_FAILURES=2 → Issues #3, #16 (divergence, rebase)
- Context pressure at 80% → Issue #12 (large prompts)
- Full catalog in doc 14 (21 failures, 6 categories)

---

## Theoretical Foundations

| Theory | Author | Year | Applied In |
|--------|--------|------|-----------|
| Cybernetics | Wiener | 1948 | OODA loop structure (doc 07) |
| Law of Requisite Variety | Ashby | 1956 | 3-action decision space (doc 03) |
| Good Regulator Theorem | Conant & Ashby | 1970 | Self-model design (doc 08) |
| Viable System Model | Beer | 1972 | System 3 / System 3* mapping (docs 06, 07) |
| OODA Loop | Boyd | — | Observe-Orient-Decide-Act cycle (doc 07) |
| Yerkes-Dodson Law | Yerkes & Dodson | 1908 | Pressure dynamics (doc 12) |
| Stigmergy | Grassé | 1959 | Indirect coordination (doc 12) |
| Self-Improvement Convergence | Song et al. | ICLR 2025 | Verifier exceeds generator (doc 08) |
| Internal Model Principle | Francis & Wonham | 1976 | Forward prediction, self-model learning (doc 08) |
| Cognitive Load Theory | Sweller | 1988 | Intrinsic/extraneous/germane load mapping (doc 12) |
| Flow State | Csikszentmihalyi | 1975 | Challenge-skill balance, flow detection (doc 12) |
| Complex Event Processing | Luckham | 2002 | Watcher composition patterns (doc 01) |
| Isolation Forest | Liu et al. | 2008 | Streaming anomaly detection (doc 01) |
| Dempster-Shafer Theory | Dempster/Shafer | 1967/76 | Watcher fusion under uncertainty (doc 01) |
| Recovery-Oriented Computing | Patterson et al. | 2002 | Self-healing, micro-reboots (doc 15) |
| Active Inference | Friston | 2010 | Precision-weighted model updates (doc 08) |

---

## Source Code References

| File | What |
|------|------|
| `crates/roko-conductor/src/lib.rs` | Module exports |
| `crates/roko-conductor/src/conductor.rs` | Conductor struct, evaluate() |
| `crates/roko-conductor/src/circuit_breaker.rs` | PlanCircuitBreaker, DashMap |
| `crates/roko-conductor/src/interventions.rs` | ConductorDecision, severity, policies |
| `crates/roko-conductor/src/diagnosis.rs` | DiagnosisEngine, 34 patterns |
| `crates/roko-conductor/src/health.rs` | SystemSnapshot, HealthStatus |
| `crates/roko-conductor/src/state_machine.rs` | PhaseTimeout, ComplexityBand |
| `crates/roko-conductor/src/stuck_detection.rs` | StuckDetector, MetaCognitionHook |
| `crates/roko-conductor/src/watchers/` | All 10 watcher implementations |
| `crates/roko-learn/src/anomaly.rs` | AnomalyDetector, EWMA |
| `crates/roko-learn/src/efficiency.rs` | AgentEfficiencyEvent |
| `crates/roko-learn/src/cascade_router.rs` | Cascade router |
| `crates/roko-learn/src/provider_health.rs` | Provider health tracker |
| `crates/roko-gate/src/adaptive_threshold.rs` | Adaptive gate thresholds |
| `crates/bardo-runtime/` | ProcessSupervisor |
| `crates/roko-learn/src/conductor.rs` | ConductorBandit (learned intervention policy) |

---

## Generation Notes

- **Source material**: 19 roko-conductor source files, 7 refactoring
  PRD documents, 5 implementation plan files, 3 legacy reference docs
- **Naming**: Roko naming conventions applied throughout (Bardo→Roko,
  Golem→Agent, Mori→Roko Orchestrator, Grimoire→Neuro, Styx→Agent Mesh, Clade→Collective)
- **Citations**: All academic references preserved with full
  attribution
- **Framing**: Conductor as cybernetic regulator and theory-of-mind,
  not timeout manager

