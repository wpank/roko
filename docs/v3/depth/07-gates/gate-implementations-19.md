# The 19 Gate Implementations

> Depth file for [07-GATES.md](../../07-GATES.md) section 2.
> Source: `crates/roko-gate/src/`

---

## 1. Overview

Roko ships 19 concrete gate implementations. They divide into rung-dispatched
gates (13 gates across 7 rungs), standalone gates (6 gates invoked outside
the rung pipeline), and 3 composition wrappers.

Every gate implements the `Gate` trait and returns `Verdict` directly -- not
`Result<Verdict>`. Every gate handles its own errors internally, converting
infrastructure failures into `Verdict::fail()`.

---

## 2. Rung-Dispatched Gates

| Rung | Index | Gate(s) | Cost | What it verifies |
|------|-------|---------|------|------------------|
| Compile | 0 | `CompileGate` | Low | Code compiles |
| Lint | 1 | `ClippyGate` | Low | No lint violations |
| Test | 2 | `TestGate` | Medium | Tests pass |
| Symbol | 3 | `SymbolGate` | Near-zero | Required symbols exist |
| GeneratedTest | 4 | `GeneratedTestGate` + `VerifyChainGate` | High | Auto-generated tests; chain verification |
| PropertyTest | 5 | `PropertyTestGate` + `FactCheckGate` | High | Property-based tests; fact-checking |
| Integration | 6 | `IntegrationGate` + `LlmJudgeGate` | Highest | Integration suite; LLM quality judgment |

---

## 3. ShellGate -- The Foundation

**File:** `crates/roko-gate/src/shell.rs`

`ShellGate` is the simplest gate and the building block for others. It runs
an arbitrary shell command and passes if the exit code is 0.

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

### The Canonical Verify Pattern

`ShellGate::verify()` demonstrates the pattern used by all shell-spawning
gates:

1. **Read payload.** Extract `GatePayload` from signal body for `working_dir`
   and `extra_env`. If absent, use process defaults.
2. **Build command.** Set program, args, cwd, env vars, `kill_on_drop(true)`.
3. **Run with timeout.** `tokio::time::timeout(duration, cmd.output()).await`
4. **Handle three outcomes:**
   - `Err(_)` -- timeout -- `Verdict::fail("timed out after N ms")`
   - `Ok(Err(io_err))` -- spawn failure -- `Verdict::fail("spawn failed: ...")`
   - `Ok(Ok(output))` -- check exit code -- `Verdict::pass()` or `Verdict::fail()`
5. **Always attach:** `.with_detail(combined_output).with_duration(elapsed_ms)`

---

## 4. CompileGate -- Rung 0

**File:** `crates/roko-gate/src/compile.rs`

Wraps `ShellGate`'s pattern with build-system awareness. Reads `BuildSystem`
from the `GatePayload` and runs the appropriate check command.

```rust
pub struct CompileGate {
    build_system: BuildSystem,
    extra_args: Vec<String>,
    timeout_ms: u64,     // default: 10 minutes (600,000 ms)
    name: String,        // e.g., "compile:cargo"
}
```

### Build System Dispatch

| BuildSystem | Command |
|-------------|---------|
| Cargo | `cargo check --workspace` |
| Npm | `npm run build` |
| Go | `go build ./...` |
| Make | `make` |

Each variant implements `program()`, `check_args()`, `test_args()`, and
`lint_args()`.

### Error Summarization

On failure, `CompileGate` extracts up to 3 error-level diagnostics from
stderr via `summarize_errors()`:

```rust
fn summarize_errors(stderr: &str, max: usize) -> String {
    let errors: Vec<&str> = stderr.lines()
        .filter(|l| l.trim_start().starts_with("error:")
                  || l.trim_start().starts_with("error["))
        .take(max)
        .collect();
    if !errors.is_empty() { errors.join("; ") }
    else { stderr.lines()
        .find(|l| !l.trim().is_empty())
        .unwrap_or("compilation failed")
        .to_string() }
}
```

This keeps the verdict's `reason` field concise while the full output lives
in `detail`.

---

## 5. ClippyGate -- Rung 1

**File:** `crates/roko-gate/src/clippy_gate.rs`

Runs the language-appropriate linter. For Rust: `cargo clippy -- -D warnings`.
For Go: `go vet`. For Node: `npm run lint`.

```rust
pub struct ClippyGate {
    build_system: BuildSystem,
    extra_args: Vec<String>,
    timeout_ms: u64,     // default: 5 minutes (300,000 ms)
    name: String,        // e.g., "clippy:cargo"
}
```

### Argument Splicing

ClippyGate handles the Cargo-specific `--` sentinel carefully: extra args
are inserted *before* the separator so they apply to the Cargo invocation
rather than to Clippy's own flags:

```rust
let dash_idx = base.iter().position(|a| *a == "--");
if let Some(idx) = dash_idx {
    for arg in &base[..idx] { cmd.arg(arg); }
    for arg in &self.extra_args { cmd.arg(arg); }
    for arg in &base[idx..] { cmd.arg(arg); }
}
```

### Design: Lint Before Test

Lint checks are fast (seconds) and catch many issues that would cause
expensive test failures (minutes). Running lint before test saves time by
failing fast.

---

## 6. TestGate -- Rung 2

**File:** `crates/roko-gate/src/test_gate.rs`

Runs the project's test suite and parses passed/failed/ignored counts from
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

```rust
pub enum TestSelector {
    All,                    // Run everything
    Quick,                  // --lib (unit tests only)
    Patterns(Vec<String>),  // Specific test patterns
}
```

For Cargo, `Quick` adds `--lib`. For Go, `Patterns(["TestFoo", "TestBar"])`
becomes `-run TestFoo|TestBar`.

### Test Count Parsing

`parse_test_counts()` dispatches by build system:

- **Cargo:** Parses `test result: ok. N passed; M failed; K ignored` lines,
  aggregating across multiple test targets.
- **Go:** Counts `--- PASS:`, `--- FAIL:`, `--- SKIP:` markers.

Parsed counts are attached to the verdict via `.with_test_count(tc)`.

---

## 7. SymbolGate -- Rung 3

**File:** `crates/roko-gate/src/symbol_gate.rs`

Unique among gates: no subprocess, no LLM calls. Parses Rust source files
directly and verifies that every symbol in a `SymbolManifest` exists with
the correct kind, visibility, and module path.

```rust
pub struct SymbolGate {
    source_roots: Vec<PathBuf>,
    name: String,
}
```

### Mismatch Taxonomy

Five mismatch categories:

| Category | Meaning | Example |
|----------|---------|---------|
| `MISSING` | Symbol not found | `struct RateLimiter at core::rate_limit` |
| `WRONG_VIS` | Found but wrong visibility | `fn check_rate (found: private, expected: pub)` |
| `WRONG_KIND` | Found but wrong item kind | `Limiter (found: struct, expected: trait)` |
| `WRONG_PATH` | Found but wrong module | `struct Clock at core::time (found at: core::clock)` |
| `AMBIGUOUS` | Multiple matches | `fn foo at core::util (2 matches)` |

### Symbol Extraction

A lightweight single-pass line-based extractor. Handles visibility
modifiers (`pub`, `pub(crate)`, `pub(super)`), modifier keywords (`async`,
`unsafe`, `const fn`), and item kinds (`struct`, `enum`, `trait`, `fn`,
`type`, `const`, `static`, `mod`).

Cost: effectively zero. No subprocess, pure file I/O.

---

## 8. DiffGate -- Vacuous Implementation Rejection

**File:** `crates/roko-gate/src/diff_gate.rs`

Solves a specific failure mode: agents that "pass" gates by producing
vacuous implementations.

### Rejection Criteria

A diff is rejected when:

1. **Empty diff:** Zero added lines.
2. **Below threshold:** Non-whitespace added lines below `min_added_lines`.
3. **All forbidden tokens:** Every substantive added line matches a forbidden
   token.

Default forbidden tokens: `todo!()`, `unimplemented!()`, `panic!("not
implemented")`, `Ok(())`, `return Ok(())`.

### Analysis

```rust
pub struct DiffAnalysis {
    pub added_lines: u32,
    pub non_whitespace_added: u32,
    pub all_added_are_forbidden: bool,
}
```

`analyze_diff()` is pure: no I/O, no subprocess. It skips diff headers
(`+++`, `---`, `@@`), counts `+` lines, filters whitespace and comments,
and checks against the forbidden token list.

---

## 9. Standalone Gates

| Gate | Module | Purpose |
|------|--------|---------|
| `CodeExecutionGate` | `code_exec.rs` | Sandboxed code execution |
| `ShellGate` | `shell.rs` | Arbitrary shell command verification |
| `BenchmarkRegressionGate` | `benchmark_gate.rs` | Criterion benchmark regression detection |
| `FormatCheckGate` | `format_check_gate.rs` | Code formatting (`cargo fmt --check`) |
| `SecurityScanGate` | `security_scan_gate.rs` | Security scanning |

---

## 10. Advanced Rung Gates

### 10.1 GeneratedTestGate (Rung 4)

Runs tests that were automatically generated by the agent or a dedicated
test-generation agent. More targeted than the project's existing test suite.

### 10.2 VerifyChainGate (Rung 4, auxiliary)

Verifies chain-related artifacts. Part of the optional chain integration.

### 10.3 PropertyTestGate (Rung 5)

Runs property-based tests (proptest style). Asserts invariants over
randomized inputs.

### 10.4 FactCheckGate (Rung 5, auxiliary)

Verifies factual claims against acceptance criteria. Pairs with
`PropertyTestGate` for combined assertion and fact verification.

### 10.5 IntegrationGate (Rung 6)

Runs the full integration test suite. Most expensive gate -- may involve
standing up services, databases, or network connections.

### 10.6 LlmJudgeGate (Rung 6, auxiliary)

The only gate that consults a model rather than a deterministic tool.
Used when properties are too nuanced for automated checking ("does this
implementation match the PRD's intent?").

---

## 11. Composition Wrappers

Three wrappers allow combining gates into richer verification topologies:

| Wrapper | Behavior |
|---------|----------|
| `ParallelGate` | Run multiple gates concurrently, collect all verdicts |
| `VotingGate` | Majority-vote across inner gates (quorum-based) |
| `FallbackGate` | Try gates in order, use first non-error verdict |

### ParallelGate

Duration is `max(durations)`, not `sum(durations)` -- gates run
concurrently. A `ParallelGate(SymbolGate, DiffGate)` completing in 50ms
and 20ms respectively finishes in 50ms total.

### VotingGate

For subjective gates (LLM judges), a single verdict is noisy. A voting
gate runs multiple judges and requires a quorum:

```rust
let pass_count = verdicts.iter().filter(|v| v.passed).count();
let passed = pass_count >= self.quorum;
```

### FallbackGate

Useful for degraded environments: try clippy, fall back to a simpler
lint if clippy is unavailable.

---

## 12. GateGenerator and Ad-Hoc Checks

`GateGenerator` / `GeneratedCheck` produce dynamically generated
verification checks at runtime, enabling the system to create task-specific
verification on the fly.

---

## 13. Shared Patterns Across All Gates

### 13.1 Builder Pattern

Every gate uses `with_*` methods for configuration:

```rust
CompileGate::cargo()
    .with_extra_args(vec!["--features".into(), "ci".into()])
    .with_timeout_ms(120_000)
```

### 13.2 GatePayload

Gates that shell out read configuration from a `GatePayload` in the signal
body:

```rust
pub struct GatePayload {
    pub working_dir: PathBuf,
    pub target_dir: Option<PathBuf>,
    pub extra_env: HashMap<String, String>,
    // ... build system, test selector, etc.
}
```

### 13.3 Timeout + kill_on_drop

Every subprocess-spawning gate sets `cmd.kill_on_drop(true)` and wraps
execution in `tokio::time::timeout()`. This ensures:

- No zombie processes from abandoned gates
- Bounded execution time for every gate

### 13.4 Error Summarization

Each gate has a gate-specific `summarize_*` function that extracts the most
relevant error lines from stderr, keeping verdict reasons concise (3 lines)
while preserving full output in `detail`.

---

## 14. Production Gate Service

The `ProductionGateService` provides the trait interface
(`ProductionGateRunner`) that the Runner-v2 event loop used and that the Graph
engine's `GatePipelineCell` (#250) calls. It composes the concrete gates with
the pipeline builder and dispatches through the production service boundary.

---

## Verification

```bash
cargo test -p roko-gate
cargo clippy -p roko-gate --no-deps -- -D warnings
```
