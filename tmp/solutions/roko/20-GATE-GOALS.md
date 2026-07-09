# Gate Pipeline: Goals

## End State Vision

Gates are composable verification building blocks that attach to any workflow
step. One dispatch path (GateService). Full adaptive intelligence on all paths.
Custom gates definable via config. Semantic verification beyond compile/test.
Self-improving thresholds that learn from every execution.

---

## 1. Unified Gate Dispatch

### Current State

Three separate gate dispatch paths exist with different capabilities:

| Path | Location | Gates | Adaptive | Feedback | Replan |
|------|----------|-------|----------|----------|--------|
| `roko run` | run.rs | 4 hardcoded | No | No | No |
| ACP runner | runner.rs | 3 hardcoded | EMA skip | No | No |
| `roko plan run` | orchestrate.rs | 7 rungs | Full SPC | Yes | Yes |

A fourth path (workflow_engine.rs) routes through GateService but is not yet
the default for all callers.

### Goal

All runtimes use GateService as the single gate dispatch point. GateConfig
is the universal interface:

```rust
pub struct GateConfig {
    pub workdir: PathBuf,
    pub enabled_gates: Vec<String>,
    pub shell_gates: Vec<ShellGateCommand>,
    pub max_rung: Option<u8>,
    // Proposed additions:
    pub complexity: Option<PlanComplexity>,
    pub prior_failures: Option<u32>,
    pub oracle_config: Option<OracleConfig>,
    pub feedback_channel: Option<FeedbackSink>,
}
```

### Key Properties

- `roko run` constructs GateConfig from roko.toml and calls GateService
- ACP runner constructs GateConfig and calls GateService
- orchestrate.rs constructs GateConfig with oracle injection and calls GateService
- workflow_engine.rs already uses GateService (keep as-is)
- Gate feedback and replan logic moves to a wrapper around GateService, not
  duplicated per caller

### Success Criteria

- `grep -rn 'fn run_gate\|fn run_gates' crates/ --include='*.rs' | grep -v target/ | grep -v test`
  returns exactly one implementation (GateService)
- All 4 paths produce identical GateReport for identical inputs
- Gate features (adaptive, feedback, replan) are configuration-driven, not path-dependent

---

## 2. Adaptive Thresholds Everywhere

### Current State

| Path | EMA | CUSUM | SPC Ensemble | Neuro Hints | Temperament | Domain Profiles |
|------|-----|-------|-------------|-------------|-------------|-----------------|
| `roko run` | No | No | No | No | No | No |
| ACP | Yes | No | No | No | No | No |
| orchestrate.rs | Yes | Yes | Yes | Yes | Yes | No |
| workflow_engine | Yes | No | No | No | No | No |

### Goal

All paths get the full adaptive intelligence stack:

1. **EMA pass-rate tracking** per rung (alpha=0.1) -- already implemented
2. **CUSUM change detection** for sustained shifts -- implemented, needs wiring
3. **SPC ensemble** (CUSUM + EWMA + BOCPD) -- implemented, alerts need draining
4. **Hotelling T-squared** for joint anomalies -- implemented, needs runtime callers
5. **Neuro hints** for knowledge-informed thresholds -- implemented, needs wiring outside orchestrate.rs
6. **Temperament-aware adjustments** -- implemented, needs wiring
7. **Domain profiles** -- implemented, needs runtime instantiation
8. **Residual-based tightening** -- implemented, needs oracle prediction residual wiring

### Success Criteria

- GateService accepts `AdaptiveThresholds` and applies all 8 features
- SPC alerts are drained and logged/surfaced after each gate pipeline run
- Hotelling T-squared is called with the full pass-rate vector after each pipeline run
- Domain profile is selected from agent role/config
- `should_skip_rung()` uses temperament-aware version

---

## 3. Gate Feedback to Agents

### Current State

`feedback_for_agent()` is called only from orchestrate.rs. It parses raw gate
output into structured `GateFeedback` with classified errors, warnings, and
suggestions. This feedback is injected into the agent's next prompt for retry.

The `roko run` and ACP paths do not feed gate results back to agents.

### Goal

All paths that dispatch agents after gate failure feed structured feedback:

1. `GateFeedback` generated after every gate pipeline run
2. Feedback severity drives retry strategy:
   - Error only -> retry with error context
   - Warning only -> retry with suggestion context
   - Mixed -> retry with full feedback
3. `compile_errors.rs` failure classification drives remediation:
   - `Retry` -> agent retries with feedback
   - `NeedsReplan` -> emit PlanReplan event
   - `Blocked` -> pause and alert
   - `NeedsHuman` -> escalate

### Success Criteria

- GateService returns `GateFeedback` alongside `GateReport`
- `feedback_for_agent()` is called from GateService, not from individual callers
- Failure classification is available on every GateVerdict

---

## 4. LLM Judge via CascadeRouter

### Current State

The LLM judge in orchestrate.rs (`AgentJudgeOracle`) goes through
`run_prepared_agent` (respects routing config) but model falls back to a
hardcoded string `claude-sonnet-4-20250514` when no model is configured.

Issues:
- Model fallback is hardcoded instead of routing through CascadeRouter
- Does not record an episode under the gate caller
- Does not track cost against a gate budget
- Not available outside orchestrate.rs

### Goal

1. LLM judge model selection goes through CascadeRouter
2. Each judge invocation records an episode (for cost tracking and learning)
3. Judge budget is configurable per plan/task
4. JudgeOracle trait is injectable into GateService via OracleConfig

### Success Criteria

- `grep -rn 'claude-sonnet-4-20250514' crates/ --type rust` returns zero hits
- LLM judge invocations appear in episode logs
- CascadeRouter selects judge model based on task complexity and budget

---

## 5. Custom Gates via Config

### Current State

GateConfig supports `enabled_gates: Vec<String>` with well-known names
(compile, clippy, test, diff, fmt, shell, judge) and `shell_gates: Vec<ShellGateCommand>`
for custom shell commands.

### Goal

Users define arbitrary verification gates in roko.toml:

```toml
[[gate.custom]]
name = "mypy-check"
program = "mypy"
args = ["--strict", "src/"]
timeout_ms = 60000
rung = 1  # run after compile, before test

[[gate.custom]]
name = "eslint-check"
program = "npx"
args = ["eslint", "--max-warnings", "0", "."]
timeout_ms = 30000
rung = 1

[[gate.custom]]
name = "integration-suite"
program = "pytest"
args = ["-x", "tests/integration/"]
timeout_ms = 300000
rung = 6

[[gate.custom]]
name = "api-health"
url = "http://localhost:8080/health"
method = "GET"
expected_status = 200
timeout_ms = 5000
rung = 6
```

Gate types:
- **Shell gates**: arbitrary programs with args and timeout
- **HTTP gates**: health checks against running services
- **MCP gates**: verification via MCP tool calls
- **Composite gates**: combine multiple gates with composition mode

### Success Criteria

- Custom gates appear in GateReport alongside built-in gates
- Custom gates respect rung ordering
- Custom gates participate in adaptive threshold tracking

---

## 6. Novel Gate Types

### 6.1 Semantic Correctness Gate

Goes beyond "does it compile?" to "does it do what was asked?":

1. Parse task description and acceptance criteria
2. Generate semantic checks from acceptance criteria
3. Verify the diff implements the described behavior
4. Score: 0.0 (no match) to 1.0 (full match)

This is a richer version of the LLM judge that uses structured acceptance
criteria rather than free-form prompting.

### 6.2 Behavioral Regression Gate

Detects behavioral changes that pass compilation but change runtime behavior:

1. Identify functions modified in the diff
2. Run existing tests that cover those functions
3. Compare test outputs (not just pass/fail) with baseline
4. Flag behavioral changes that were not explicitly requested

### 6.3 Dependency Analysis Gate

Verifies dependency hygiene:

1. Check for circular dependencies introduced by the change
2. Verify semver compatibility of new dependencies
3. Detect vendored code that duplicates existing dependencies
4. Flag security advisories on new dependencies (via `cargo audit`)

### 6.4 API Compatibility Gate

For library crates, verify API surface stability:

1. Compare public API before and after the change
2. Flag breaking changes (removed public items, changed signatures)
3. Verify deprecation annotations for breaking changes
4. Check that docs exist for new public items

### 6.5 Performance Gate

Compare performance metrics before and after:

1. Run benchmarks on affected code paths
2. Compare against baseline (stored in `.roko/learn/benchmarks.json`)
3. Flag regressions exceeding a configurable threshold (e.g., 10%)
4. Record new baselines for new benchmarks

### Success Criteria

- At least one novel gate type (semantic correctness) is live and producing
  meaningful verdicts
- Novel gates integrate with the adaptive threshold system
- Novel gates produce `GateFeedback` that agents can act on

---

## 7. Gate Composition at Runtime

### Current State

Composition wrappers (ParallelGate, VotingGate, FallbackGate) exist but are
only used in tests. ComposedGatePipeline supports 4 composition modes but is
not constructed by any runtime code path.

### Goal

Gate composition is configurable and used at runtime:

```toml
[gate.pipeline]
mode = "sequential"  # or "parallel", "voting", "fallback"

# Voting mode: 2 of 3 LLM judges must agree
[[gate.pipeline.voting]]
gates = ["judge-1", "judge-2", "judge-3"]
threshold = 0.67

# Fallback mode: try cargo test, fall back to shell test
[[gate.pipeline.fallback]]
primary = ["test:cargo"]
fallback = ["test:shell"]
```

### Success Criteria

- ComposedGatePipeline is constructed from roko.toml config
- Voting mode is used for LLM judge consensus (N-of-M agreement)
- Fallback mode is used for degraded-mode verification

---

## 8. Process Reward Model Integration

### Current State

ProcessRewardModel (`process_reward.rs`) is built with TurnSnapshot, Promise,
and Progress signals. Not connected to the orchestrator loop.

### Goal

PRM drives early termination and intervention decisions:

1. After each agent turn, record a TurnSnapshot (highest rung, verdicts, error count, diff lines)
2. Compute Promise (probability of eventual success)
3. Compute Progress (delta from previous turn)
4. If Promise < threshold -> abandon task, replan
5. If Progress stalls for N turns -> change model/strategy
6. PRM history feeds into episode logs for learning

### Success Criteria

- TurnSnapshot is recorded after each gate pipeline run in orchestrate.rs
- Promise/Progress values are logged to efficiency events
- Low Promise triggers task abandonment with replan
- Stalled Progress triggers model/strategy change via CascadeRouter

---

## 9. UX Data Feeds

From v2 UX showcase scenarios, the gate pipeline must produce these data feeds:

### GateRow (all scenarios)

Live gate result strip below messages:
- Per-gate: name, status (pending/running/passed/failed/skipped), detail, duration
- Pulsing amber dot when running

### GateMark (pipeline, incident, router scenarios)

Per-line gutter marks in code editor:
- line_number, kind (ok/error/warn/info), message
- Derived from compile_errors.rs structured output

### SwarmGates (tournament scenario)

Per-agent array of gate results:
- 3 gate dots per parallel agent (compile/test/clippy)
- Independently tracked per agent

### ComparisonTable (tournament scenario)

Per-approach performance comparison:
- Approach name, p99 latency, complexity score, risk score, cost
- Auto-generated from benchmark gate + test gate results

### GateThresholdUpdate (pipeline scenario)

Adaptive threshold change notifications:
- Rung, old threshold, new threshold, reason
- Derived from AdaptiveThresholds.observe() changes

### Data Feed Implementation

```rust
pub enum GateEvent {
    GateStarted { gate_name: String, rung: u8 },
    GatePassed { gate_name: String, rung: u8, duration_ms: u64, detail: String },
    GateFailed { gate_name: String, rung: u8, duration_ms: u64, feedback: GateFeedback },
    GateSkipped { gate_name: String, rung: u8, reason: String },
    ThresholdUpdated { rung: u8, old: f64, new: f64, reason: String },
    PipelineCompleted { report: GateReport, promise: f64, progress: f64 },
}
```

### Success Criteria

- GateService emits GateEvents via EventBus during execution
- TUI and SSE consumers can render GateRow, GateMark from events
- Threshold updates are surfaced as events

---

## 10. Self-Improving Gate System

### Long-Term Vision

The gate pipeline becomes self-improving through feedback loops:

1. **Learning from verdicts**: gate pass/fail patterns inform CascadeRouter
   model selection (low test pass rate -> use stronger model)
2. **Learning from feedback**: common error patterns inform prompt enrichment
   (agent keeps making lifetime errors -> add lifetime guidance to prompt)
3. **Learning from thresholds**: adaptive thresholds inform complexity
   classification (tasks that consistently fail at Standard -> auto-escalate to Complex)
4. **Learning from PRM**: Promise/Progress trajectories inform budget allocation
   (tasks with steep Promise decline -> allocate more budget early)

### Feedback Loops (in order of implementation priority)

1. Gate failure classification -> error_patterns -> prompt enrichment (partial, via playbooks)
2. Adaptive thresholds -> complexity auto-escalation (implemented, escalation ladder)
3. SPC alerts -> CascadeRouter model change (not implemented)
4. PRM trajectories -> budget allocation (not implemented)
5. Neuro knowledge -> gate threshold hints (implemented, orchestrate.rs only)
6. Domain profiles -> per-role gate configuration (built, not instantiated)

---

## Sources

All goals derived from analysis of:

- `crates/roko-gate/src/` -- 40 source files, ~20.1K LOC
- `crates/roko-gate/tests/` -- 4 integration test files
- `crates/roko-cli/src/orchestrate.rs` -- Gate pipeline integration
- `crates/roko-cli/src/run.rs` -- Simple gate dispatch
- `crates/roko-acp/src/runner.rs` -- ACP gate dispatch
- `crates/roko-runtime/src/workflow_engine.rs` -- Workflow engine gate dispatch
- `crates/roko-core/src/foundation.rs` -- GateRunner, GateConfig, GateReport traits
- `tmp/solutions/runner/LESSONS.md` -- Anti-pattern checks from mega-parity runs
