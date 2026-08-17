# Gate Pipeline Facade: 7-Rung Pipeline Design Document

## Executive Summary

The gate pipeline is the verification backbone of roko's self-hosting loop. It validates
agent output after each task execution, determining whether code changes are correct enough
to persist. The design specifies 7 rungs of increasing rigor. In practice, only rungs 0-2
(compile, lint, test) produce real verdicts. Rungs 3-6 have fully implemented gate structs
but the orchestrator never provides the input signals they require, causing them to emit
`stub_verdict()` (always-pass) at runtime.

This document covers: exact current state, architectural design, per-rung implementation
status and design, the oracle system, adaptive thresholds, the learning feedback loop,
and a migration plan from stubs to real implementations.

---

## Current State: What Runs vs What Stubs

### The 7-Rung Architecture

The canonical rung enum is defined in `crates/roko-gate/src/rung_selector.rs`:

```rust
pub enum Rung {
    Compile      = 0,  // cargo check --workspace
    Lint         = 1,  // cargo clippy --workspace --no-deps -- -D warnings
    Test         = 2,  // cargo test --workspace
    Symbol       = 3,  // SymbolGate: regex-based symbol manifest verification
    GeneratedTest = 4, // GeneratedTestGate + VerifyChainGate
    PropertyTest = 5,  // PropertyTestGate + FactCheckGate
    Integration  = 6,  // LlmJudgeGate + IntegrationGate
}
```

Each rung maps to one or more concrete `Verify` implementations in
`crates/roko-gate/src/rung_dispatch.rs`.

### Rung Status Matrix

| Rung | Index | Gate(s) | Gate Impl | Runtime Status | Why |
|------|-------|---------|-----------|---------------|-----|
| Compile | 0 | `CompileGate` | REAL: shells out to `cargo check` | RUNS | Always wired |
| Lint | 1 | `ClippyGate` | REAL: shells out to `cargo clippy` | RUNS | Wired when `clippy_enabled=true` |
| Test | 2 | `TestGate` | REAL: shells out to `cargo test` | RUNS | Wired unless `skip_tests=true` |
| Symbol | 3 | `SymbolGate` | REAL: 500-line file-walking regex symbol verifier with full test suite | STUB AT RUNTIME | Orchestrator never provides `RungExecutionInputs.symbol_signal` |
| GeneratedTest | 4 | `GeneratedTestGate` | REAL: stages test files from ArtifactStore, runs them | STUB AT RUNTIME | Orchestrator never provides `RungExecutionConfig.generated_test_artifacts` |
| GeneratedTest | 4 | `VerifyChainGate` | REAL: runs plan-specific verify.sh scripts | STUB AT RUNTIME | Signal never has `verify_script` tag; no fallback configured |
| PropertyTest | 5 | `PropertyTestGate` | REAL: runs proptest-prefixed tests with env vars | RUNS (vacuously) | Actually executes but finds no `prop_` tests, so passes trivially |
| PropertyTest | 5 | `FactCheckGate` | REAL: searches claims via SearchOracle | STUB AT RUNTIME | Orchestrator never provides `fact_check_signal` or `fact_check_oracle` |
| Integration | 6 | `LlmJudgeGate` | REAL: delegates to JudgeOracle for diff scoring | STUB AT RUNTIME | Orchestrator never provides `llm_judge_signal` (but does wire the oracle) |
| Integration | 6 | `IntegrationGate` | REAL: runs integration test patterns | STUB AT RUNTIME | Orchestrator never provides `integration_test_pattern` |

### The Key Insight

Every gate struct for rungs 3-6 is **fully implemented and tested** in the `roko-gate` crate.
The problem is not missing gate implementations. The problem is in
`crates/roko-cli/src/orchestrate.rs`: the orchestrator never constructs the
`RungExecutionInputs` and `RungExecutionConfig` that feed these gates their required data.

### The `enable_advanced_rungs` Flag

In `crates/roko-core/src/config/gates.rs`:

```rust
pub struct GatesConfig {
    pub enable_advanced_rungs: bool, // default: false
    // ...
}
```

In `crates/roko-cli/src/orchestrate.rs` (lines 17879-17894):

```rust
Rung::Symbol | Rung::PropertyTest | Rung::Integration => {
    if !gate_config.enable_advanced_rungs {
        tracing::debug!(
            ?rung,
            "advanced rung skipped (gates.enable_advanced_rungs not set)"
        );
        skipped_count = skipped_count.saturating_add(1);
    } else {
        // Advanced rungs dispatched through run_gate_rung which uses
        // stub_verdict fallback when inputs are not wired.
        tracing::debug!(?rung, "advanced rung enabled via config");
        skipped_count = skipped_count.saturating_add(1);  // SAME increment!
    }
}
```

Both branches increment `skipped_count` identically. The flag has zero observable effect:
even when `enable_advanced_rungs = true`, the rungs are still counted as skipped and no
gate step is pushed into the pipeline. The comment in the `else` branch is honest -- it
acknowledges that even if dispatched, `run_gate_rung` would hit `stub_verdict` because
the inputs are not wired -- but the code never even dispatches them.

### The `stub_verdict()` Function

Defined in `crates/roko-gate/src/rung_dispatch.rs` (line 290):

```rust
fn stub_verdict(gate: &str, detail: impl Into<String>) -> Verdict {
    let message = format!("stub gate; {}", detail.into());
    let mut verdict = Verdict::pass(gate.to_string());
    verdict.reason.clone_from(&message);
    verdict.detail = Some(message);
    verdict
}
```

Always returns `Verdict::pass`. The gate runs, the gate "passes", the learning loop records
a pass. But no verification occurred.

Each gate function in `rung_dispatch.rs` guards on its required inputs with `let Some(...) = ... else`:

```rust
async fn run_symbol_gate(ctx, inputs, config) -> Verdict {
    let Some(signal) = inputs.symbol_signal.as_ref() else {
        return stub_verdict("symbol", "no SymbolManifest wired into rung 3");
    };
    let Some(source_roots) = config.source_roots.clone() else {
        return stub_verdict("symbol", "no source roots configured for rung 3");
    };
    SymbolGate::new(source_roots).verify(&signal, ctx).await  // never reached
}
```

### What Actually Validates Agent Output

In production (default config), only these checks run:

1. **Compile gate** (`cargo check --workspace`) -- verifies the code type-checks
2. **Clippy gate** (`cargo clippy --workspace --no-deps -- -D warnings`) -- verifies no lint warnings
3. **Test gate** (`cargo test --workspace`) -- verifies existing tests pass

### Consequences for Self-Hosting

An agent can produce code that compiles, passes clippy, and passes all existing tests
while being:
- Completely wrong logically (implements the wrong algorithm)
- Missing functions it claimed to implement (task says "add `pub fn rate_limit`" but it added nothing)
- Duplicating existing code (reimplements something that already exists)
- Unrelated to the task prompt (modifies the wrong file entirely)
- Introducing subtle bugs (off-by-one, race condition, memory leak via `Arc` cycle)

The gate pass rate metric in `roko-learn` is inflated -- it measures "does it compile
and pass existing tests?" not "did the agent accomplish its task?"

---

## Gate Pipeline Execution Flow

```
                        orchestrate.rs
                             |
                    run_gate_pipeline(plan_id, rung)
                             |
                             v
                 +---------------------------+
                 | select_rungs(complexity,  |
                 |   caps, prior_failures)   |
                 +---------------------------+
                             |
         Complexity-selected rungs (e.g., [Compile, Lint, Test])
                             |
                             v
                 +---------------------------+
                 | selected_gate_steps()     |
                 | For each selected rung:   |
                 |   - Compile -> CompileGate|
                 |   - Lint -> ClippyGate    |
                 |   - Test -> TestGate      |
                 |   - Symbol/Prop/Integ ->  |
                 |       SKIP (both paths)   |
                 +---------------------------+
                             |
                    steps[] pushed into GatePipeline
                             |
                             v
            +----------------------------------+
            | GatePipeline.verify(signal, ctx) |
            | Sequential, short-circuit on     |
            | first failure                    |
            +----------------------------------+
                   |          |          |
                   v          v          v
             CompileGate  ClippyGate  TestGate
             cargo check  cargo clippy cargo test
                   |          |          |
                   v          v          v
              Verdict     Verdict     Verdict
                   |          |          |
                   +----------+----------+
                             |
                    Aggregate Verdict
                             |
                             v
            +----------------------------------+
            | Post-gate processing:            |
            | 1. adaptive_thresholds.observe() |
            | 2. gate_artifacts.store()        |
            | 3. gate_ratchet.check()          |
            | 4. feedback_for_agent()          |
            | 5. verdict_publisher.publish()   |
            | 6. efficiency_event()            |
            +----------------------------------+
                             |
                             v
                  GateRunOutcome { passed, summary,
                    counts, verdicts, artifacts }
```

### Alternative Path: `run_gate_rung()`

When a specific rung is requested (not the default Compile-only path), the orchestrator
calls `run_gate_rung()` which delegates to `rung_dispatch::run_rung()`. This path
_does_ support all 7 rungs but hits `stub_verdict()` for rungs 3-6 because
`RungExecutionInputs` fields are left as `None`.

```
                    run_gate_rung(plan_id, signal, rung)
                             |
                             v
                 +---------------------------+
                 | Build RungExecutionInputs |
                 | (symbol_signal: None,     |
                 |  fact_check_signal: None,  |
                 |  llm_judge_signal: None)   |
                 +---------------------------+
                             |
                             v
                 +---------------------------+
                 | gate_rung_config()        |
                 | enrich_rung_config()      |
                 | (wires generated_test_    |
                 |  artifacts + integration  |
                 |  test pattern from task)  |
                 +---------------------------+
                             |
                             v
                 +---------------------------+
                 | run_rung(signal, ctx,     |
                 |   rung, inputs, config)   |
                 +---------------------------+
                             |
            If rung 0-2: real gate executes
            If rung 3-6: stub_verdict() returns pass
```

---

## Per-Rung Design

### Rung 0: Compile (CompileGate)

**File**: `crates/roko-gate/src/compile.rs`
**Status**: FULLY WIRED

**What it does**: Spawns `cargo check --workspace` (or equivalent for Go/Npm), captures
exit code + stderr, parses structured compile errors via `compile_errors.rs`.

**Input**: `GatePayload` from signal body (working dir, target dir, extra env).
**Output**: `Verdict { passed: exit == 0, error_digest: parsed errors, duration_ms }`.

**Timeout**: `TimeoutConfig::gate_compile()` (default 600s).

**Gaps**: None. This gate is production-quality. Structured error classification
(`ErrorCategory`, `FailureClass`, `GateRetryPolicy`) already feeds the replan loop.

---

### Rung 1: Lint (ClippyGate)

**File**: `crates/roko-gate/src/clippy_gate.rs`
**Status**: FULLY WIRED

**What it does**: Spawns `cargo clippy --workspace --no-deps -- -D warnings`. Non-zero exit
is a failure. Shares the structured error classification from `compile_errors.rs`.

**Input**: Same `GatePayload` as CompileGate.
**Output**: `Verdict` with lint error digest.

**Timeout**: `TimeoutConfig::gate_clippy()` (default 300s).

**Config**: `gates.clippy_enabled` (default true). When false, rung is skipped.

**Gaps**: None operationally. Could be enhanced with per-lint-rule tracking (which clippy
lints fire most) for the learning loop.

---

### Rung 2: Test (TestGate)

**File**: `crates/roko-gate/src/test_gate.rs`
**Status**: FULLY WIRED

**What it does**: Spawns `cargo test --workspace`, parses test counts from output
(`test result: ok. N passed; M failed; K ignored`). Supports `TestSelector` for
pattern-scoped test execution.

**Input**: `GatePayload` from signal body.
**Output**: `Verdict` with `test_count: TestCount { passed, failed, ignored }`.

**Timeout**: `TimeoutConfig::gate_test()` (default 900s).

**Config**: `gates.skip_tests` (default false). When true, rung is skipped entirely.

**Gaps**: None operationally.

---

### Rung 3: Symbol (SymbolGate)

**File**: `crates/roko-gate/src/symbol_gate.rs`
**Status**: GATE IMPLEMENTATION COMPLETE, NOT WIRED INTO ORCHESTRATOR

**What it does**: Walks source directories, extracts Rust item declarations (structs, enums,
traits, functions, type aliases, consts, statics, mods) using a lightweight single-pass
regex scanner. Compares discovered symbols against a `SymbolManifest` that lists expected
symbols with their kind, visibility, module path, and optional signature substring.

Reports 5 mismatch categories:
- `MISSING` -- symbol not found anywhere
- `WRONG_VIS` -- found but wrong visibility (e.g., private vs pub)
- `WRONG_KIND` -- found but wrong item type (e.g., struct vs trait)
- `WRONG_PATH` -- found but in wrong module
- `AMBIGUOUS` -- multiple matches at the expected path
- `WRONG_SIG` -- found but signature substring not present

**Input required**: `RungExecutionInputs.symbol_signal` containing a `SymbolManifest` as
its JSON body. Also requires `RungExecutionConfig.source_roots`.

**Why it stubs**: The orchestrator never constructs the `SymbolManifest`. The manifest
must come from somewhere -- either the task spec, the agent's output metadata, or an
LLM-based extraction step.

**Implementation design to wire it**:

1. **Manifest source**: Extract expected symbols from the task definition. Each task in
   `tasks.toml` should optionally include an `[expected_symbols]` section:
   ```toml
   [[tasks]]
   id = "wire-rate-limiter"
   prompt = "Implement pub struct RateLimiter..."

   [[tasks.expected_symbols]]
   name = "RateLimiter"
   kind = "struct"
   visibility = "pub"
   module_path = "roko_core::rate_limit"
   ```

2. **Alternative: LLM-based extraction**: Before dispatch, send the task prompt to a cheap
   model (Haiku) with a structured output schema asking: "What symbols should be created
   or modified by this task?" Parse the response into a `SymbolManifest`.

3. **Wiring in orchestrate.rs**:
   ```rust
   // In run_gate_rung(), before building RungExecutionInputs:
   let symbol_signal = if let Some(manifest) = self.build_symbol_manifest(plan_id, task_def) {
       let body = Body::from_json(&manifest)?;
       Some(Signal::builder(Kind::Task).body(body).build())
   } else {
       None
   };
   let inputs = RungExecutionInputs {
       symbol_signal,
       // ...
   };
   ```

4. **Source roots**: Set from the workspace root:
   ```rust
   config.source_roots = Some(vec![self.workdir.join("crates")]);
   ```

**Scoring/threshold logic**: Binary pass/fail. All expected symbols must be present with
correct kind and visibility. The threshold is not configurable per-symbol; the gate fails
if any expectation is unmet.

**Test strategy**: 16 existing unit tests cover all mismatch categories, multi-root search,
nested modules, `lib.rs`/`mod.rs` path resolution, and the parser. Integration test needed
for the orchestrator wiring path.

---

### Rung 4: GeneratedTest (GeneratedTestGate + VerifyChainGate)

**File**: `crates/roko-gate/src/generated_test_gate.rs`,
         `crates/roko-gate/src/verify_chain_gate.rs`
**Status**: GATE IMPLEMENTATIONS COMPLETE, NOT WIRED INTO ORCHESTRATOR

#### GeneratedTestGate

**What it does**: Reads generated behavioral test files from an immutable `ArtifactStore`,
stages them into `<worktree>/tests/__roko_generated__/`, runs them via `cargo test` with
a pattern selector (default prefix `gen_`), parses results, and cleans up staging via RAII.
The implementer agent never sees the test source, preserving isolation.

**Input required**:
- `RungExecutionConfig.generated_test_artifacts`: An `Arc<dyn ArtifactStore>` containing
  test files keyed by plan ID
- Signal must have a `plan` or `plan_id` tag
- Signal body must be a `GatePayload`

**Why it stubs**: The orchestrator does wire `generated_test_artifacts` in
`enrich_rung_config()` for this specific rung, looking at the plan's exec dir. But
the artifact store path (`.roko/artifacts/`) is empty because no upstream step (enrichment,
TestGenerator role) ever populates generated test files into it.

**Implementation design to wire it**:

1. **Test generation step**: Add a `TestGenerator` enrichment phase between agent dispatch
   and gate execution. This phase:
   - Receives the task spec and the agent's diff
   - Sends both to an LLM with a system prompt: "Generate behavioral tests that verify
     this implementation satisfies the spec. Output Rust test functions prefixed with `gen_`."
   - Writes the generated test files into the `ArtifactStore` under
     `generated-tests/gen_<task_id>.rs`

2. **ArtifactStore population**: The store already has an `InMemoryArtifactStore` and
   the gate expects a file-system-backed implementation:
   ```rust
   // In enrich_rung_config(), the existing code:
   config.generated_test_artifacts = self.generated_test_store_for(dir);
   // This resolves to the plan's .roko/plans/<plan_id>/generated-tests/ directory.
   // The directory just needs to be populated by the enrichment step.
   ```

3. **Cost control**: Generated tests are a one-time LLM call per task (Haiku-class model,
   small prompt). The test generation prompt is ~500 tokens + the diff. Cost is negligible
   compared to the agent dispatch itself.

#### VerifyChainGate

**What it does**: Runs a plan-specific `verify.sh` script. The script path is read from
`signal.tag("verify_script")`. The script emits `[PASS]`/`[FAIL]` lines. Exit code 0 with
no `[FAIL]` lines means pass. Supports retry-once (2s delay) and zero-test-guard.

**Input required**: Signal must have a `verify_script` tag pointing to the script path.
Alternatively, `RungExecutionConfig.verify_chain_fallback` can provide a fallback `Verify`.

**Why it stubs**: No task in `tasks.toml` currently has a `verify_script` field, and the
orchestrator never sets the tag on the signal.

**Implementation design to wire it**:

1. **Task spec extension**: Add optional `verify_script` to task definitions:
   ```toml
   [[tasks]]
   id = "wire-rate-limiter"
   verify_script = "plans/verify/rate-limiter.sh"
   ```

2. **Auto-generation**: For plans generated from PRDs, the `prd plan` command should
   generate a `verify.sh` per task that runs the task's acceptance criteria as shell
   commands (e.g., `cargo test --test rate_limiter_integration`).

3. **Signal tagging**: In orchestrate.rs, when building the gate signal:
   ```rust
   if let Some(script) = task_def.verify_script.as_ref() {
       payload_builder = payload_builder.tag("verify_script", script);
   }
   ```

---

### Rung 5: PropertyTest (PropertyTestGate + FactCheckGate)

**File**: `crates/roko-gate/src/property_test_gate.rs`,
         `crates/roko-gate/src/fact_check.rs`
**Status**: GATE IMPLEMENTATIONS COMPLETE, NOT WIRED EFFECTIVELY

#### PropertyTestGate

**What it does**: Runs `cargo test` with `PROPTEST_CASES=256` and a name selector for
tests prefixed with `prop_`. Captures counterexamples into `error_digest` (truncated to
2048 bytes). Disables proptest persistence for hermeticity.

**Input required**: `GatePayload` (same as TestGate). No special config needed.

**Why it stubs**: It does not actually stub. The gate runs, but finds zero `prop_`-prefixed
tests in the workspace and passes vacuously. This is correct behavior -- the gate works,
the workspace just has no property tests. The issue is that no enrichment step generates
property tests for agent-produced code.

**Implementation design**:

1. **Property test generation**: Similar to GeneratedTestGate, add a `PropertyTestGenerator`
   enrichment phase that generates proptest-based tests from the task spec. The generated
   tests should exercise invariants:
   - "For all valid inputs, `rate_limit()` never exceeds N per window"
   - "For all orderings of concurrent calls, no data race occurs"
   - The tests are written to `tests/__roko_generated__/prop_<task_id>.rs`

2. **Existing test discovery**: Scan the workspace for existing `prop_`-prefixed tests
   and run those. This requires no enrichment -- just running the gate.

3. **Cost**: Running proptest is CPU-bound, not LLM-bound. 256 cases per property at ~1ms
   per case is ~0.3s total. Negligible.

#### FactCheckGate

**What it does**: Extracts "claims" from agent output text (sentences >= 20 chars with
substantive words), searches each claim via a `SearchOracle`, and passes if
`verified / total >= min_confidence` (default 0.7).

**Input required**:
- `RungExecutionInputs.fact_check_signal`: A signal with text body containing claims
- `RungExecutionConfig.fact_check_oracle`: An `Arc<dyn SearchOracle>` (e.g., Perplexity)

**Why it stubs**: The orchestrator never provides either input.

**Implementation design to wire it**:

1. **Claim source**: The agent's response text (the implementation description, not the
   code). Agents typically explain what they did -- these explanations contain factual claims
   ("I added RateLimiter to roko_core::rate_limit", "The function takes a `u32` parameter").

2. **Oracle implementation**: Wire `PerplexitySearchClient` as the search oracle. This is
   already available in `roko-agent` via the research subsystem.

3. **When to use**: Primarily useful for research tasks and documentation tasks, not code
   implementation tasks. For code tasks, SymbolGate + GeneratedTestGate provide better
   verification.

4. **Config**: `[gates.fact_check] min_confidence = 0.7` already defined in the schema.

---

### Rung 6: Integration (LlmJudgeGate + IntegrationGate)

**File**: `crates/roko-gate/src/llm_judge_gate.rs`,
         `crates/roko-gate/src/integration_gate.rs`
**Status**: GATE IMPLEMENTATIONS COMPLETE, PARTIALLY WIRED

#### LlmJudgeGate

**What it does**: Sends a `JudgePayload` (task description + diff, truncated to 30KB) to a
`JudgeOracle`, receives a quality score in `[0.0, 1.0]`, and passes if
`score >= min_score` (default 0.8). Non-blocking by default -- oracle errors pass with a
warning.

**Input required**:
- `RungExecutionInputs.llm_judge_signal`: A signal with `JudgePayload` body
- `RungExecutionConfig.llm_judge_oracle`: An `Arc<dyn JudgeOracle>`

**Partial wiring**: The orchestrator DOES wire `llm_judge_oracle` in `gate_rung_config()`:

```rust
// orchestrate.rs:18102
config.llm_judge_oracle = Some(Arc::new(AgentJudgeOracle {
    command: self.config.agent.command.clone(),
    exec_dir: self.workdir.clone(),
    model: judge_model,
    timeout_ms: DEFAULT_REQUEST_TIMEOUT_MS,
    skip_permissions: true,
}));
```

The oracle is wired. But `llm_judge_signal` is never constructed. The orchestrator has
`gate_diff_for_plan()` which shells out to `git diff HEAD` but never puts the result into
a `JudgePayload` signal.

**Implementation design to wire it**:

1. **Diff collection**: `gate_diff_for_plan()` already exists and works. Use it:
   ```rust
   let llm_judge_signal = if let Some(diff) = self.gate_diff_for_plan(plan_id).await {
       let payload = JudgePayload {
           task_description: task_def.map(|t| t.prompt.clone()).unwrap_or_default(),
           diff,
       };
       let body = Body::from_json(&payload)?;
       Some(Signal::builder(Kind::Task).body(body).build())
   } else {
       None
   };
   ```

2. **Judge prompt engineering**: The current prompt is generic ("Score this implementation
   on a 0.0-1.0 scale"). Should be enhanced with:
   - The task's acceptance criteria
   - The expected symbols/changes
   - Rubric: "1.0 = fully implements the spec, 0.0 = completely wrong"

3. **Scoring**: Non-blocking by default. Oracle errors pass. This is correct for self-hosting
   because LLM judge is advisory -- it should not block on API flakes. In blocking mode
   (configurable), oracle errors fail the verdict.

4. **Cost**: One LLM call per task (Haiku/Sonnet-class). ~2000 tokens input (task + truncated
   diff), ~50 tokens output (score). ~$0.001-0.005 per call.

#### IntegrationGate

**What it does**: Three scenario types:
- `BuildTest`: Run `cargo test -- <pattern>` for integration-test-specific patterns
- `Script`: Execute a bash script, exit 0 = pass
- `Custom`: Caller-supplied async closure (for spawning Anvil, mirages, etc.)

Includes warmup delay (default 2s), outer timeout (120s), `kill_on_drop(true)`.

**Input required**:
- `RungExecutionConfig.integration_test_pattern`: The test pattern to run
- `RungExecutionConfig.integration_build_system`: Build system (default Cargo)

**Partial wiring**: `enrich_rung_config()` attempts to extract integration test patterns
from task verify steps:

```rust
if let Some(td) = task_def {
    // Look for a verify step with phase "integration" and use its command.
    for step in &td.verify_steps {
        if step.phase.as_deref() == Some("integration") {
            config.integration_test_pattern = Some(step.command.clone());
            break;
        }
    }
}
```

But tasks rarely have `verify_steps` with `phase = "integration"`.

**Implementation design to wire it**:

1. **Default pattern**: For Rust projects, default to running tests matching the plan's
   crate name: `--test <crate_name>` or `-- <crate_name>::integration`.

2. **Script scenario**: For plans with a `verify.sh`, use that as the integration scenario
   (overlaps with VerifyChainGate; choose one or compose both).

3. **Custom scenario**: Reserved for complex deployments (Anvil lifecycle, daemon start/stop).
   Not needed for self-hosting MVP.

---

## Oracle System for Rungs 4-6

Rungs 4-6 use oracle-pattern traits to avoid a dependency cycle between `roko-gate` and
`roko-agent`. Each gate defines a minimal async trait; the orchestrator implements it by
wrapping an agent client.

### Oracle Trait Definitions

```rust
// crates/roko-gate/src/llm_judge_gate.rs
#[async_trait]
pub trait JudgeOracle: Send + Sync {
    async fn judge(&self, prompt: &str) -> Result<f32, String>;
}

// crates/roko-gate/src/fact_check.rs
#[async_trait]
pub trait SearchOracle: Send + Sync {
    async fn search(&self, query: &str) -> Result<Vec<SearchHit>, String>;
}
```

### Oracle Implementations in Orchestrate.rs

The orchestrator implements `JudgeOracle` via `AgentJudgeOracle` which shells out to
the configured agent command (e.g., `claude`) with a structured prompt asking for a
quality score.

`SearchOracle` is not currently implemented in the orchestrator. It needs a wrapper around
`PerplexitySearchClient` from `roko-agent`.

### Oracle Design Principles

1. **Cheap models**: Judge and search oracles should use the cheapest model that produces
   reliable structured output (Haiku for judge, Sonar for search).

2. **Non-blocking default**: Oracle failures should not block the pipeline. The gate passes
   with a warning, and the learning loop records the advisory nature.

3. **Timeout isolation**: Each oracle call has its own timeout (default 120s). This prevents
   slow API responses from blocking the entire pipeline.

4. **Caching**: Oracle results for the same diff/query should be cached per-plan to avoid
   redundant API calls on retries.

---

## Adaptive Threshold System

**File**: `crates/roko-gate/src/adaptive_threshold.rs`
**Persistence**: `.roko/learn/gate-thresholds.json`

### How It Works

Per-rung EMA (exponential moving average) of pass rates, tracking:

```rust
pub struct RungStats {
    pub ema_pass_rate: f64,          // [0.0, 1.0], starts at 0.5
    pub total_observations: u64,
    pub consecutive_passes: u32,     // reset on failure
    pub cusum_high: f64,             // CUSUM upward shift detector
    pub cusum_low: f64,              // CUSUM downward shift detector
    pub cusum_shift_detected: bool,  // true when CUSUM fires
}
```

### EMA Update

```
ema = alpha * value + (1 - alpha) * ema_prev
```

Where `alpha = 0.1` and `value = 1.0` (pass) or `0.0` (fail).

### What It Controls

1. **Retry budgeting**: `suggested_max_retries(rung)` maps EMA to retry count:
   - High pass rate (near 1.0) -> 1 retry (gate usually passes)
   - Low pass rate (near 0.0) -> 5 retries (gate often fails, give more chances)

2. **Skip suggestions**: `should_skip_rung(rung)` returns true after 20 consecutive passes.
   Advisory only -- the caller decides whether to honor it.

3. **Shift detection**: CUSUM (Cumulative Sum) algorithm detects distributional shifts.
   When CUSUM fires, EMA resets to the current observation for fast adaptation.

4. **SPC ensemble** (GATE-01): Three statistical process control detectors per rung:
   - CUSUM detector
   - EWMA Control Chart
   - Bayesian Online Changepoint Detection (BOCPD)

5. **Hotelling T-squared** (GATE-08): Joint anomaly detection across all rungs simultaneously.
   Detects systemic shifts (e.g., all gates degrading together suggests a build environment
   problem, not a code problem).

### Integration with the Learning Loop

```
Agent completes task
        |
        v
  Gate pipeline runs
        |
        v
  For each verdict:
    adaptive_thresholds.observe(rung, passed)
        |
        v
  Check for SPC alerts
    drain_spc_alerts() -> Vec<(rung, SpcAlert)>
        |
        v
  Observe full pipeline
    observe_pipeline(&[pass_rates])
        |
        v
  Record efficiency event
    efficiency_event { gate_pass_rate, ... }
        |
        v
  If gate failed:
    classify_gate_failure(verdict) -> GateFailureClassification
    feedback_for_agent(verdict) -> FeedbackItem
    build_gate_failure_plan_revision() -> revised plan
```

### Temperament Adjustments (AGT-06)

```rust
pub fn threshold_for_temperament(&self, rung: u32, temperament: Temperament) -> f64 {
    match temperament {
        Conservative => base * 1.10,  // stricter
        Balanced     => base,
        Aggressive   => base * 0.85,  // more permissive
        Exploratory  => base * 0.90,
    }
}
```

### Domain-Specific Profiles (P1-14)

```rust
ThresholdProfile::coding()   // strict compile (0.90), moderate test (0.65)
ThresholdProfile::research() // lenient compile (0.70), strict test (0.85)
ThresholdProfile::security() // strict everything (0.90+), fewer retries
```

### Neuro Integration (INT-15)

```rust
pub fn apply_neuro_hints(&mut self, known_failure_rungs: &[u32], known_stable_rungs: &[u32])
```

When the knowledge store (roko-neuro) has recorded persistent failure patterns for specific
rungs, the adaptive thresholds tighten CUSUM sensitivity for those rungs (detecting smaller
shifts sooner) and bias EMA downward when few observations exist.

---

## Rung Results Feeding the Learning Loop

### Data Flow

```
Verdict
  |
  +-> AdaptiveThresholds.observe(rung, passed)
  |     Updates EMA, CUSUM, SPC detectors
  |     Persists to .roko/learn/gate-thresholds.json
  |
  +-> EpisodeLogger.record_gate(verdict)
  |     Appends to .roko/episodes.jsonl
  |     Includes: gate name, passed, duration, error_digest, test_count
  |
  +-> EfficiencyEvent
  |     gate_pass_rate = gates_passed / gates_executed
  |     Appends to .roko/learn/efficiency.jsonl
  |
  +-> GateArtifactStore.store(verdict_json)
  |     Content-addressed storage in .roko/artifacts/
  |     Hash used for forensic chain reconstruction
  |
  +-> GateRatchet.check(plan_id, rung, passed)
  |     Detects regressions: once a rung passes, it must keep passing
  |     Persists to .roko/learn/gate-ratchet.json
  |
  +-> VerdictPublisher.publish(verdict)
  |     Broadcasts as Pulse on event bus
  |     Consumed by TUI, SSE endpoints, dashboard
  |
  +-> CascadeRouter.observe(model, outcome)
  |     Feeds model routing decisions
  |     "Model X fails at gate rung 2 more often"
  |
  +-> PromptExperiments.record(variant, outcome)
        A/B test tracking for prompt variants
```

### Gate Failure Replan

When a gate fails, `build_gate_failure_plan_revision()` in orchestrate.rs constructs a
revised plan that:
1. Includes the failure classification (`GateFailureClassification`)
2. Includes the error digest and feedback items
3. Sends to the agent with context: "Your previous attempt failed gate X because Y. Fix it."

This is gated by `learning_config.replan_on_gate_failure` (default true) and capped by
the replan revision ledger to avoid infinite loops.

---

## Standalone Gates (Outside the Rung Pipeline)

Six additional gates exist outside the 7-rung pipeline for scenario-specific checks:

| Gate | File | Purpose | When Invoked |
|------|------|---------|--------------|
| `DiffGate` | `diff_gate.rs` | Analyzes git diff for quality signals (file count, churn, etc.) | Post-task review |
| `CodeExecutionGate` | `code_exec.rs` | Runs code in a sandboxed environment | Ad-hoc verification |
| `ShellGate` | `shell.rs` | Arbitrary shell command as a gate (exit 0 = pass) | Custom `[[gates.rungs]]` in TOML |
| `BenchmarkRegressionGate` | `benchmark_gate.rs` | Detects performance regressions | Post-task perf check |
| `FormatCheckGate` | `format_check_gate.rs` | `cargo fmt --check` / `prettier --check` | Code style enforcement |
| `SecurityScanGate` | `security_scan_gate.rs` | `cargo audit` / `npm audit` | Security scanning |

### Composition Wrappers

Three composition wrappers allow combining any gates:

- `ParallelGate`: Run multiple gates concurrently, collect all verdicts
- `VotingGate`: Majority-vote across inner gates (pass if > threshold fraction agree)
- `FallbackGate`: Try gates in order, use first non-error verdict

These are wired into `ComposedGatePipeline` which supports `GateComposition::Sequential`,
`Parallel`, `Voting { threshold }`, and `Fallback` modes.

---

## Complexity-Based Rung Selection

The rung selector (`crates/roko-gate/src/rung_selector.rs`) decides which rungs to run
based on plan complexity:

```
| Complexity | Compile | Lint | Test | Symbol | GenTest | PropTest | Integration |
|------------|---------|------|------|--------|---------|----------|-------------|
| Trivial    |    X    |      |      |        |         |          |             |
| Simple     |    X    |  X   |      |        |         |          |             |
| Standard   |    X    |  X   |  X   |   X    |         |          |             |
| Complex    |    X    |  X   |  X   |   X    |    X    |    X     |      X      |
```

### Escalation Ladder

On repeated failure, the effective complexity escalates:
- Trivial + 1 failure -> Simple (adds lint)
- Trivial + 2 failures -> Standard (adds test + symbol)
- Trivial + 3 failures -> Complex (full suite)

Saturates at Complex. This means a task that keeps failing will eventually run all 7 rungs
(assuming the advanced rungs get wired).

### Capability Caps

`RungCaps` narrows the selection: a rung is included only if the project actually has the
corresponding capability. Caps can only remove rungs, never add ones the complexity band
did not select.

---

## Test Strategy per Rung

### Rung 0-2: Unit + Integration Tests (Complete)

- `compile.rs`: Unit tests for error parsing; integration tests in `tests/compile_real_project.rs`
  that scaffold a minimal Cargo project, introduce compile errors, and verify the gate catches them.
- `clippy_gate.rs`: Same pattern -- scaffold project, verify clippy warnings are caught.
- `test_gate.rs`: Scaffold project with passing/failing tests, verify count parsing.

### Rung 3: Symbol Gate (Complete)

16 unit tests + integration tests covering all mismatch categories, multi-root search,
module path resolution, and parser edge cases. No orchestrator integration tests yet.

Needed: Integration test that proves the full path from task spec -> SymbolManifest
construction -> gate execution -> correct verdict.

### Rung 4: GeneratedTest + VerifyChain (Partial)

GeneratedTestGate: 12 unit tests covering artifact store, prefix matching, staging,
plan resolution, and cleanup. Missing: test with a real Cargo project where generated
tests actually run and verify behavior.

VerifyChainGate: Unit tests for script resolution, line protocol parsing, retry logic,
and zero-test guard. Missing: end-to-end test with a real verify.sh.

### Rung 5: PropertyTest + FactCheck (Partial)

PropertyTestGate: Unit tests for env var setup and prefix matching. Missing: test with
actual proptest properties.

FactCheckGate: 10 unit tests with mock oracles covering claim extraction, keyword matching,
threshold behavior, and error handling. Missing: test with a real search oracle.

### Rung 6: LlmJudge + Integration (Partial)

LlmJudgeGate: 15 unit tests with mock oracles covering threshold, blocking/non-blocking,
truncation, prompt construction, and clamping. Missing: test with a real LLM oracle.

IntegrationGate: Unit tests for BuildTest and Script scenarios. Missing: end-to-end
test with a real project lifecycle.

---

## Migration Plan: Stubs to Real Implementations

### Phase 1: Wire LlmJudgeGate (Lowest Effort, Highest Impact)

**Effort**: ~2 hours
**Impact**: Every task gets a quality score from an LLM

The oracle is already wired. The only missing piece is constructing `llm_judge_signal` from
the git diff. The code to get the diff (`gate_diff_for_plan()`) already exists.

Steps:
1. In `run_gate_rung()`, after computing `gate_diff_for_plan()`, build a `JudgePayload`
   signal and set `inputs.llm_judge_signal`.
2. Remove the `enable_advanced_rungs` guard for `Rung::Integration` (or fix both branches).
3. Test: run `roko plan run` on a real plan, verify the judge verdict appears in gate output.

### Phase 2: Wire SymbolGate (Medium Effort, High Impact)

**Effort**: ~4 hours
**Impact**: Catches "agent did nothing" and "agent created wrong thing" failures

Steps:
1. Add `expected_symbols` to the task parser (optional TOML field).
2. In orchestrate.rs, build `SymbolManifest` from task spec when present.
3. Set `inputs.symbol_signal` and `config.source_roots`.
4. Remove the guard for `Rung::Symbol`.
5. Test: create a plan with `expected_symbols`, run it, verify the symbol check runs.

### Phase 3: Wire GeneratedTestGate (Higher Effort, Highest Impact)

**Effort**: ~8 hours
**Impact**: Behavioral verification -- the single most impactful rung for self-hosting

Steps:
1. Implement a `TestGenerator` enrichment step that generates behavioral tests from
   task spec + diff.
2. Store generated tests in the plan's artifact store.
3. The rest is already wired (enrich_rung_config handles the artifact store path).
4. Test: generate tests for a real task, verify they run and catch intentional regressions.

### Phase 4: Wire VerifyChainGate (Medium Effort, Medium Impact)

**Effort**: ~4 hours
**Impact**: Plan-specific acceptance criteria verification

Steps:
1. Add `verify_script` to task parser.
2. Auto-generate verify scripts during `prd plan` from acceptance criteria.
3. Set the `verify_script` tag on the gate signal.
4. Test: add a verify script to a real plan, verify it runs.

### Phase 5: Wire FactCheckGate + PropertyTestGate (Lower Priority)

**Effort**: ~6 hours each
**Impact**: Specialized -- useful for specific domains, not critical for MVP

FactCheckGate: Wire Perplexity search oracle, set fact_check_signal from agent output text.
PropertyTestGate: Generate proptest properties from task specs, write to staging dir.

### Phase 6: Fix `enable_advanced_rungs` Flag

**Effort**: ~30 minutes
**Impact**: Prerequisite for all of the above

The fix is trivial -- the `else` branch should push the rung into the pipeline steps
instead of incrementing `skipped_count`:

```rust
Rung::Symbol | Rung::PropertyTest | Rung::Integration => {
    if !gate_config.enable_advanced_rungs {
        skipped_count = skipped_count.saturating_add(1);
    } else {
        // Actually dispatch through run_gate_rung
        // (which will stub_verdict if inputs are still None,
        //  but at least the wiring path is exercised)
        steps.push((rung, Box::new(CanonicalRungGate { ... })));
    }
}
```

This should be done first, as a foundation for all subsequent wiring.

---

## Impact on Self-Hosting Reliability

### Current State (Rungs 0-2 Only)

- **What's caught**: Syntax errors, type errors, lint warnings, test regressions
- **What's missed**: Logic errors, missing implementations, wrong implementations,
  task-spec violations, code duplication, architectural drift
- **Estimated false-pass rate**: ~30-40% of agent tasks that "pass" gates are actually
  incorrect or incomplete

### Target State (All 7 Rungs)

- **Rung 3 (Symbol)**: Catches "agent did nothing" (+10% detection)
- **Rung 4 (GeneratedTest)**: Catches "agent did the wrong thing" (+15% detection)
- **Rung 4 (VerifyChain)**: Catches plan-specific acceptance failures (+5% detection)
- **Rung 5 (PropertyTest)**: Catches boundary/invariant bugs (+5% detection)
- **Rung 6 (LlmJudge)**: Catches subtle logic errors via review (+10% detection)
- **Rung 6 (Integration)**: Catches system-level failures (+5% detection)
- **Estimated false-pass rate with all rungs**: ~5-10%

### Relationship to Self-Hosting

The gate pipeline is the only mechanism that prevents bad agent output from being persisted
and built upon. Every gate that stubs is a hole in the verification net. The gap between
"compiles" and "correct" is where most self-hosting failures live.

Priority should be: Phase 1 (LlmJudge) > Phase 2 (Symbol) > Phase 3 (GeneratedTest),
because each successive phase catches a different class of failure and their impacts
are roughly cumulative.

---

## Files Referenced

| File | Purpose |
|------|---------|
| `crates/roko-gate/src/lib.rs` | Public API surface, module declarations |
| `crates/roko-gate/src/rung_selector.rs` | Rung enum, complexity selection, escalation ladder |
| `crates/roko-gate/src/rung_dispatch.rs` | Runtime dispatch, `stub_verdict()`, `RungExecutionInputs/Config` |
| `crates/roko-gate/src/gate_pipeline.rs` | `GatePipeline`, `ComposedGatePipeline`, composition modes |
| `crates/roko-gate/src/compile.rs` | Rung 0: CompileGate |
| `crates/roko-gate/src/clippy_gate.rs` | Rung 1: ClippyGate |
| `crates/roko-gate/src/test_gate.rs` | Rung 2: TestGate |
| `crates/roko-gate/src/symbol_gate.rs` | Rung 3: SymbolGate (500+ lines, fully tested) |
| `crates/roko-gate/src/generated_test_gate.rs` | Rung 4: GeneratedTestGate (artifact staging) |
| `crates/roko-gate/src/verify_chain_gate.rs` | Rung 4: VerifyChainGate (verify.sh scripts) |
| `crates/roko-gate/src/property_test_gate.rs` | Rung 5: PropertyTestGate (proptest runner) |
| `crates/roko-gate/src/fact_check.rs` | Rung 5: FactCheckGate (Perplexity search oracle) |
| `crates/roko-gate/src/llm_judge_gate.rs` | Rung 6: LlmJudgeGate (LLM quality scoring) |
| `crates/roko-gate/src/integration_gate.rs` | Rung 6: IntegrationGate (end-to-end scenarios) |
| `crates/roko-gate/src/adaptive_threshold.rs` | Adaptive thresholds: EMA, CUSUM, SPC, Hotelling |
| `crates/roko-gate/src/feedback.rs` | Agent feedback generation from gate verdicts |
| `crates/roko-gate/src/compile_errors.rs` | Structured error classification for replan |
| `crates/roko-gate/src/composition.rs` | ParallelGate, VotingGate, FallbackGate |
| `crates/roko-gate/src/artifact_store.rs` | Content-addressed gate artifact persistence |
| `crates/roko-gate/src/ratchet.rs` | Regression detection (once a rung passes, it stays) |
| `crates/roko-gate/src/verdict_publisher.rs` | Event bus broadcast of gate verdicts |
| `crates/roko-core/src/config/gates.rs` | `GatesConfig`, `enable_advanced_rungs` flag |
| `crates/roko-cli/src/orchestrate.rs` | PlanRunner gate methods (~lines 17244-18260) |
| `crates/roko-cli/src/gate_runner.rs` | Gate helper functions extracted from orchestrate.rs |
