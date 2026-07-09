# Gate Pipeline: Implementation Plan

Phased plan to converge gate dispatch, wire unused features, and add novel
gate types. Each phase is independently valuable and does not block later phases.

---

## Phase 1: Converge Gate Dispatch (I-1, I-4, I-5)

**Goal**: All gate dispatch paths use GateService. Gate feedback is built into
the service layer, not per-caller.

### 1.1 Extend GateConfig

**File**: `crates/roko-core/src/foundation.rs`

Add fields to GateConfig:

```rust
pub struct GateConfig {
    pub workdir: PathBuf,
    pub enabled_gates: Vec<String>,
    pub shell_gates: Vec<ShellGateCommand>,
    pub max_rung: Option<u8>,
    // New fields:
    pub complexity: Option<PlanComplexity>,
    pub prior_failures: Option<u32>,
}
```

Complexity and prior_failures enable GateService to perform rung selection
internally rather than requiring each caller to do it. When None, GateService
runs all enabled gates without selection.

**Effort**: Small. Add 2 optional fields, default to None.

### 1.2 Add Feedback to GateReport

**File**: `crates/roko-core/src/foundation.rs`

```rust
pub struct GateReport {
    pub verdicts: Vec<GateVerdict>,
    // New:
    pub feedback: Option<GateFeedback>,
    pub failure_classification: Option<GateFailureClassification>,
}
```

GateService generates feedback and classification internally after running
all gates. Callers get structured feedback without calling feedback_for_agent()
themselves.

**Effort**: Small. Add 2 fields, construct in GateService.run_gates().

### 1.3 Wire GateService into run.rs

**File**: `crates/roko-cli/src/run.rs`

Replace the `run_gate()` function that matches on GateConfig variants with:

```rust
async fn run_gate(config: &RokoConfig, workdir: &Path) -> Result<GateReport> {
    let gate_config = GateConfig {
        workdir: workdir.into(),
        enabled_gates: config.gate.enabled.clone(),
        shell_gates: config.gate.shell_gates.clone(),
        max_rung: config.gate.max_rung,
        complexity: None,
        prior_failures: None,
    };

    let thresholds = AdaptiveThresholds::load_or_new(&thresholds_path());
    let svc = GateService::new().with_adaptive_thresholds(thresholds);
    let report = svc.run_gates(gate_config).await?;

    // Save updated thresholds
    if let Some(adaptive) = &svc.adaptive {
        adaptive.lock().map_err(|_| ...)?.save(&thresholds_path())?;
    }

    Ok(report)
}
```

This gives `roko run` adaptive thresholds, rung ordering, and gate feedback
for free.

**Effort**: Medium. Replace ~40 lines of match logic with GateService call.

### 1.4 Wire GateService into ACP runner

**File**: `crates/roko-acp/src/runner.rs`

Replace `run_gates()` that hardcodes compile -> test -> clippy with:

```rust
async fn run_gates(&self) -> Result<GateReport> {
    let gate_config = GateConfig {
        workdir: self.workdir.clone(),
        enabled_gates: vec!["compile".into(), "clippy".into(), "test".into()],
        shell_gates: vec![],
        max_rung: None,
        complexity: None,
        prior_failures: None,
    };

    let thresholds = AdaptiveThresholds::load_or_new(&self.thresholds_path());
    let svc = GateService::new().with_adaptive_thresholds(thresholds);
    let report = svc.run_gates(gate_config).await?;

    // Save thresholds (existing logic)
    ...

    Ok(report)
}
```

This fixes I-5 (clippy ordering) automatically -- GateService orders by rung.

**Effort**: Medium. Replace ~60 lines with GateService call.

### 1.5 Wire Feedback into GateService

**File**: `crates/roko-gate/src/gate_service.rs`

After all verdicts are collected, generate feedback:

```rust
// At end of run_gates(), before Ok(GateReport):
let feedback = if let Some(failing_verdict) = verdicts.iter().find(|v| !v.passed && !v.skipped) {
    Some(feedback_for_agent(&failing_verdict.output, rung_for_name(&failing_verdict.gate_name)?))
} else {
    None
};

let classification = feedback.as_ref().and_then(|fb| {
    if !fb.passed {
        Some(classify_gate_failure(&fb.errors.join("\n")))
    } else {
        None
    }
});

Ok(GateReport { verdicts, feedback, failure_classification: classification })
```

**Effort**: Small. 15 lines of feedback generation.

### 1.6 Verification

After Phase 1, verify:

```bash
# Only one gate dispatch implementation
grep -rn 'fn run_gate\b' crates/roko-cli/src/run.rs --include='*.rs' | wc -l
# Should be 0 (replaced by GateService call)

grep -rn 'fn run_gates\b' crates/roko-acp/src/runner.rs --include='*.rs' | wc -l
# Should be 0 (replaced by GateService call)

# Feedback available on all paths
cargo test --workspace -p roko-gate
cargo test --workspace -p roko-cli -- gate
cargo test --workspace -p roko-acp -- gate
```

**Total Phase 1 Effort**: 1-2 days.

---

## Phase 2: Fix Stub Verdicts and LLM Judge (I-2, I-3)

### 2.1 Replace Stub Pass with Stub Skip

**File**: `crates/roko-gate/src/rung_dispatch.rs`

Change stub_verdict from pass to skip:

```rust
fn stub_verdict(gate: &str, detail: impl Into<String>) -> Verdict {
    let message = format!("stub gate; {}", detail.into());
    // Before: Verdict::pass(gate.to_string())
    // After: explicit skip marker
    Verdict::skip(gate, message)
}
```

If `Verdict` doesn't have a `skip()` constructor, add one to roko-core:

```rust
impl Verdict {
    pub fn skip(gate: impl Into<String>, reason: impl Into<String>) -> Self {
        Self {
            passed: false,
            gate: gate.into(),
            reason: reason.into(),
            skipped: true,
            ..Default::default()
        }
    }
}
```

Or alternatively, keep using `Verdict::pass()` but tag the verdict to indicate
it was a stub. The key requirement is that stub verdicts are distinguishable
from real passes in GateReport and adaptive threshold tracking.

**Impact**: Stub verdicts no longer inflate the "all gates passed" count.
Orchestrators can distinguish "5/5 passed" from "3 passed, 2 skipped".

**Effort**: Small. Change one function, add one constructor.

### 2.2 Route LLM Judge Through CascadeRouter

**File**: `crates/roko-cli/src/orchestrate.rs`

Replace hardcoded model fallback:

```rust
// Before:
let model = self.config.agent.model.as_deref()
    .unwrap_or("claude-sonnet-4-20250514");

// After:
let model = self.cascade_router
    .select_model("gate-judge", task_complexity)
    .unwrap_or_else(|| self.config.agent.model.clone()
        .unwrap_or_else(|| "claude-sonnet-4-20250514".into()));
```

And record an episode for each judge invocation:

```rust
let episode = Episode::new("gate-judge")
    .with_model(&model)
    .with_tokens(response_tokens)
    .with_cost(estimated_cost);
self.episode_logger.record(episode)?;
```

**Effort**: Medium. Requires CascadeRouter access from the gate context.

### 2.3 Add Gate Budget Tracking

**File**: `crates/roko-cli/src/orchestrate.rs`

Track cumulative gate cost per task:

```rust
struct GateBudget {
    max_judge_invocations: u32,
    current_invocations: u32,
    max_cost_usd: f64,
    current_cost_usd: f64,
}

impl GateBudget {
    fn can_invoke_judge(&self) -> bool {
        self.current_invocations < self.max_judge_invocations
            && self.current_cost_usd < self.max_cost_usd
    }
}
```

When budget is exhausted, the LLM judge gate returns a budget-exceeded skip
verdict instead of invoking the oracle.

**Effort**: Small. New struct + budget check before judge call.

### 2.4 Verification

```bash
# No hardcoded model strings
grep -rn 'claude-sonnet-4-20250514' crates/ --include='*.rs' | grep -v test
# Should be 0

# Judge invocations appear in episodes
cargo run -p roko-cli -- plan run ... # then check .roko/episodes.jsonl
```

**Total Phase 2 Effort**: 1-2 days.

---

## Phase 3: Wire Adaptive Intelligence (I-6, I-7, I-8)

### 3.1 Drain SPC Alerts After Pipeline Run

**File**: `crates/roko-gate/src/gate_service.rs`

After running all gates, drain SPC alerts and include them in the report:

```rust
// In run_gates(), after the verdict loop:
let spc_alerts = if let Some(adaptive) = &self.adaptive {
    if let Ok(mut thresholds) = adaptive.lock() {
        thresholds.drain_spc_alerts()
    } else {
        vec![]
    }
} else {
    vec![]
};

Ok(GateReport {
    verdicts,
    feedback,
    failure_classification,
    spc_alerts,  // New field
})
```

**Effort**: Small. Drain and include in report.

### 3.2 Call Hotelling observe_pipeline After Full Run

**File**: `crates/roko-gate/src/gate_service.rs`

After all verdicts are collected, feed the pass-rate vector to Hotelling:

```rust
// After the verdict loop:
let pass_rates: Vec<f64> = verdicts.iter()
    .filter(|v| !v.skipped)
    .map(|v| if v.passed { 1.0 } else { 0.0 })
    .collect();

if let Some(adaptive) = &self.adaptive {
    if let Ok(mut thresholds) = adaptive.lock() {
        thresholds.observe_pipeline(&pass_rates);
        if thresholds.joint_anomaly_detected() {
            // Include joint anomaly alert in report
        }
    }
}
```

**Effort**: Small. 10 lines after existing loop.

### 3.3 Instantiate Domain Profiles

**File**: `crates/roko-gate/src/gate_service.rs` or caller

When constructing AdaptiveThresholds for a new agent/plan, initialize from
the appropriate domain profile:

```rust
impl AdaptiveThresholds {
    pub fn from_profile(profile: &ThresholdProfile) -> Self {
        let mut at = Self::new();
        for (&rung, &prior) in &profile.rung_priors {
            let stats = at.rungs.entry(rung).or_default();
            stats.ema_pass_rate = prior;
        }
        if let Some(sensitivity) = profile.cusum_sensitivity_override {
            at.cusum_sensitivity = sensitivity;
        }
        at
    }
}
```

Callers select profile by agent role:

```rust
let profile = match agent_role {
    "implementer" | "coder" => ThresholdProfile::coding(),
    "researcher" => ThresholdProfile::research(),
    "auditor" | "security" => ThresholdProfile::security(),
    _ => ThresholdProfile::by_name("default"),
};
let thresholds = AdaptiveThresholds::from_profile(&profile);
```

**Effort**: Medium. New constructor + caller changes.

### 3.4 Wire Temperament to Gate Decisions

**File**: `crates/roko-gate/src/gate_service.rs`

GateService should accept a temperament parameter and use temperament-aware
skip and threshold methods:

```rust
pub struct GateService {
    adaptive: Option<Arc<Mutex<AdaptiveThresholds>>>,
    temperament: Temperament,  // New
}

fn should_skip_rung_adaptively(&self, rung: Option<u8>) -> Result<bool> {
    // Use temperament-aware version
    thresholds.should_skip_rung_for_temperament(u32::from(r), self.temperament)
}
```

**Effort**: Small. Add field, use temperament-aware methods.

### 3.5 Verification

```bash
# SPC alerts in report
cargo test --workspace -p roko-gate -- spc
# Hotelling called at runtime
cargo test --workspace -p roko-gate -- hotelling
# Profile initialization
cargo test --workspace -p roko-gate -- profile
```

**Total Phase 3 Effort**: 1-2 days.

---

## Phase 4: Wire Failure Classification (I-10)

### 4.1 Route by Failure Action

**File**: `crates/roko-cli/src/orchestrate.rs`

After gate failure, use the failure classification to determine next action:

```rust
match report.failure_classification {
    Some(classification) => {
        match classification.recommended_action {
            GateFailureAction::Retry => {
                // Existing retry logic with feedback injection
                inject_gate_feedback(agent_context, &report.feedback);
                retry_task(task_id);
            }
            GateFailureAction::NeedsReplan => {
                // Don't retry -- emit replan event immediately
                build_gate_failure_plan_revision(task_id, &classification);
            }
            GateFailureAction::Blocked => {
                // Pause task, alert operator
                pause_task(task_id, &classification);
                emit_alert("Gate blocked", &classification);
            }
            GateFailureAction::NeedsHuman => {
                // Escalate -- stop retrying, request human input
                escalate_task(task_id, &classification);
            }
        }
    }
    None => {
        // Fallback: retry with feedback (existing behavior)
        retry_task(task_id);
    }
}
```

**Effort**: Medium. Refactor retry logic into action-based dispatch.

### 4.2 Track Failure Patterns

Wire error_patterns.rs into the learning subsystem:

```rust
// After each gate failure:
let patterns = records_from_classification(&classification);
for pattern in patterns {
    self.error_pattern_store.record(pattern);
}

// Periodically:
let frequent_patterns = self.error_pattern_store.top_k(10);
// Inject frequent patterns into prompt enrichment as warnings
```

**Effort**: Medium. Wire existing modules together.

### 4.3 Verification

```bash
# Different failure types get different actions
cargo test --workspace -p roko-cli -- gate_failure_action
# Error patterns recorded
cargo test --workspace -p roko-learn -- error_pattern
```

**Total Phase 4 Effort**: 1-2 days.

---

## Phase 5: Process Reward Model (I-12)

### 5.1 Record TurnSnapshot Per Agent Turn

**File**: `crates/roko-cli/src/orchestrate.rs`

After each gate pipeline run in the agent loop:

```rust
let snapshot = TurnSnapshot {
    rung: highest_passing_rung(&report),
    verdicts: report.verdicts.iter().map(verdict_to_core).collect(),
    error_count: report.feedback.map_or(0, |fb| fb.errors.len() as u32),
    diff_lines: count_diff_lines(&agent_output),
};

prm.history.push(snapshot);
```

### 5.2 Compute Promise and Progress

```rust
let promise = prm.promise();   // Probability of eventual success
let progress = prm.progress(); // Delta from previous turn

// Log to efficiency events
efficiency_logger.log_prm(task_id, turn, promise, progress);
```

### 5.3 Act on PRM Signals

```rust
if promise < ABANDON_THRESHOLD {
    // Task is unlikely to succeed -- abandon and replan
    abandon_task(task_id, "low promise");
    build_replan_from_failure(task_id);
} else if progress < STALL_THRESHOLD && prm.history.len() >= 3 {
    // Task is stalling -- change strategy
    cascade_router.switch_model(task_id, "stalled");
}
```

### 5.4 Verification

```bash
# PRM signals in efficiency events
cargo run -p roko-cli -- learn efficiency | grep prm
```

**Total Phase 5 Effort**: 1-2 days.

---

## Phase 6: Novel Gate Types

### 6.1 Semantic Correctness Gate

**New file**: `crates/roko-gate/src/semantic_gate.rs`

Uses the EvalGenerator to create task-specific verification checks:

```rust
pub struct SemanticCorrectnessGate {
    oracle: Arc<dyn JudgeOracle>,
    acceptance_criteria: Vec<String>,
}

impl Verify for SemanticCorrectnessGate {
    async fn verify(&self, signal: &Engram, ctx: &Context) -> Verdict {
        let diff = extract_diff(signal);
        let scores: Vec<f32> = Vec::new();

        for criterion in &self.acceptance_criteria {
            let prompt = format!(
                "Does this code change satisfy the criterion: '{}'?\n\nDiff:\n{}\n\nScore 0.0-1.0:",
                criterion, diff
            );
            let score = self.oracle.judge(&prompt).await?;
            scores.push(score);
        }

        let avg_score = scores.iter().sum::<f32>() / scores.len() as f32;
        if avg_score >= 0.7 {
            Verdict::pass("semantic").with_score(avg_score)
        } else {
            Verdict::fail("semantic", format!("score {avg_score:.2} below threshold"))
        }
    }
}
```

### 6.2 Incremental Compile Gate

**Modified file**: `crates/roko-gate/src/compile.rs`

Add crate-scoped compilation:

```rust
impl CompileGate {
    pub fn cargo_package(package: &str) -> Self {
        Self::new(BuildSystem::Cargo)
            .with_extra_args(vec!["--package".into(), package.into()])
    }

    pub fn cargo_packages(packages: &[String]) -> Self {
        let mut args = Vec::new();
        for pkg in packages {
            args.push("--package".into());
            args.push(pkg.clone());
        }
        Self::new(BuildSystem::Cargo).with_extra_args(args)
    }
}
```

Callers determine affected crates from the diff and only compile those.

### 6.3 Dependency Analysis Gate

**New file**: `crates/roko-gate/src/dependency_gate.rs`

```rust
pub struct DependencyGate {
    baseline_lockfile: PathBuf,
}

impl Verify for DependencyGate {
    async fn verify(&self, signal: &Engram, ctx: &Context) -> Verdict {
        // 1. Parse Cargo.lock diff
        // 2. Identify new/changed/removed dependencies
        // 3. Run cargo audit on new dependencies
        // 4. Check for circular dependency introduction
        // 5. Verify semver compatibility
    }
}
```

### 6.4 Flaky Test Detector

**Modified file**: `crates/roko-gate/src/test_gate.rs`

```rust
impl TestGate {
    pub fn with_flaky_detection(mut self, retry_count: u32) -> Self {
        self.flaky_retries = retry_count;
        self
    }
}

// In verify():
if !verdict.passed && self.flaky_retries > 0 {
    // Re-run failed tests up to flaky_retries times
    let mut flaky_passes = 0;
    for _ in 0..self.flaky_retries {
        let retry = self.run_tests(signal, ctx).await;
        if retry.passed { flaky_passes += 1; }
    }
    if flaky_passes > 0 {
        // Test is flaky -- pass with warning
        verdict = Verdict::pass("test:cargo")
            .with_detail(format!("flaky: passed {flaky_passes}/{} retries", self.flaky_retries));
    }
}
```

**Total Phase 6 Effort**: 3-5 days.

---

## Phase 7: Custom Gates from Config

### 7.1 Gate Config Schema

**File**: `crates/roko-core/src/config/mod.rs` (or new gate config module)

```rust
#[derive(Deserialize)]
pub struct CustomGateConfig {
    pub name: String,
    pub program: String,
    pub args: Vec<String>,
    pub timeout_ms: u64,
    pub rung: Option<u8>,
    pub fail_on_stderr: bool,
}

#[derive(Deserialize)]
pub struct HttpGateConfig {
    pub name: String,
    pub url: String,
    pub method: String,
    pub expected_status: u16,
    pub timeout_ms: u64,
    pub rung: Option<u8>,
}
```

### 7.2 Gate Construction from Config

**File**: `crates/roko-gate/src/gate_service.rs`

```rust
impl GateService {
    pub fn with_custom_gates(mut self, gates: Vec<CustomGateConfig>) -> Self {
        for gate_cfg in gates {
            self.custom_gates.insert(gate_cfg.name.clone(), gate_cfg);
        }
        self
    }
}

// In gate_for_name():
if let Some(custom) = self.custom_gates.get(name) {
    return Some(Box::new(
        ShellGate::new(&custom.program, custom.args.clone())
            .with_timeout_ms(custom.timeout_ms)
            .with_name(&custom.name)
    ));
}
```

**Total Phase 7 Effort**: 2-3 days.

---

## Phase 8: Gate Events for UX

### 8.1 GateEvent Enum

**File**: `crates/roko-gate/src/gate_service.rs` or new events module

```rust
#[derive(Clone, Debug, Serialize)]
pub enum GateEvent {
    GateStarted { gate_name: String, rung: u8 },
    GatePassed { gate_name: String, rung: u8, duration_ms: u64, detail: String },
    GateFailed { gate_name: String, rung: u8, duration_ms: u64, errors: Vec<String> },
    GateSkipped { gate_name: String, rung: u8, reason: String },
    ThresholdUpdated { rung: u8, old_threshold: f64, new_threshold: f64, trigger: String },
    PipelineCompleted { passed: bool, duration_ms: u64, gates_run: usize, gates_skipped: usize },
    SpcAlert { rung: u8, alert: SpcAlert },
    JointAnomaly { t_squared: f64, threshold: f64 },
}
```

### 8.2 Event Emission from GateService

```rust
impl GateService {
    pub fn with_event_sink(mut self, sink: mpsc::Sender<GateEvent>) -> Self {
        self.event_sink = Some(sink);
        self
    }

    fn emit(&self, event: GateEvent) {
        if let Some(sink) = &self.event_sink {
            let _ = sink.try_send(event);
        }
    }
}

// In run_gates() loop:
self.emit(GateEvent::GateStarted { gate_name: gate_name.clone(), rung: r });
let verdict = gate.verify(&signal, &ctx).await;
if verdict.passed {
    self.emit(GateEvent::GatePassed { ... });
} else {
    self.emit(GateEvent::GateFailed { ... });
}
```

### 8.3 TUI/SSE Consumers

TUI GateRow renders from GateEvent stream:

```rust
match event {
    GateEvent::GateStarted { gate_name, .. } => {
        gate_row.set_status(&gate_name, GateStatus::Running);
    }
    GateEvent::GatePassed { gate_name, duration_ms, .. } => {
        gate_row.set_status(&gate_name, GateStatus::Passed);
        gate_row.set_duration(&gate_name, duration_ms);
    }
    GateEvent::GateFailed { gate_name, duration_ms, errors, .. } => {
        gate_row.set_status(&gate_name, GateStatus::Failed);
        gate_row.set_detail(&gate_name, &errors[0]);
    }
    // ...
}
```

**Total Phase 8 Effort**: 2-3 days.

---

## Implementation Priority

| Phase | Effort | Impact | Priority |
|-------|--------|--------|----------|
| Phase 1: Converge dispatch | 1-2 days | High (eliminates 3 paths) | P0 |
| Phase 2: Fix stubs + judge | 1-2 days | High (fixes AP-1, AP-5) | P0 |
| Phase 3: Wire adaptive intelligence | 1-2 days | Medium (enables SPC, Hotelling) | P1 |
| Phase 4: Failure classification | 1-2 days | Medium (smarter retry/replan) | P1 |
| Phase 5: Process reward model | 1-2 days | Medium (early termination) | P2 |
| Phase 6: Novel gate types | 3-5 days | Medium (semantic, incremental) | P2 |
| Phase 7: Custom gates from config | 2-3 days | Medium (user extensibility) | P2 |
| Phase 8: Gate events for UX | 2-3 days | Medium (real-time visibility) | P2 |

**Total estimated effort**: 12-20 days across all phases.

Phases 1-2 are the critical path: they eliminate the duplicate dispatch paths and
fix the most impactful anti-patterns (stub pass, hardcoded model). Everything
after Phase 2 builds on the unified GateService foundation.

---

## Dependencies

- Phase 1 is independent (can start immediately)
- Phase 2 depends on Phase 1 (GateService is the single dispatch point)
- Phases 3-8 depend on Phase 1 (GateService extensions)
- Phases 3-8 are independent of each other (can be done in any order)
- Phase 5 (PRM) benefits from Phase 4 (failure classification feeds PRM)
- Phase 8 (events) benefits from Phase 3 (SPC alerts are events)

---

## Verification Checkpoints

After each phase, verify with:

```bash
# Compile
cargo check --workspace

# Lint
cargo clippy --workspace --no-deps -- -D warnings

# Tests
cargo test --workspace

# Integration: run through actual gate pipeline
cargo run -p roko-cli -- run "add a comment to lib.rs"
cargo run -p roko-cli -- plan run plans/ --resume .roko/state/executor.json

# Anti-pattern checks
grep -rn 'fn run_gate\b\|fn run_gates\b' crates/ --include='*.rs' | grep -v target/ | grep -v test | grep -v gate_service
# Should return 0 hits after Phase 1

grep -rn 'claude-sonnet-4-20250514' crates/ --include='*.rs' | grep -v test
# Should return 0 hits after Phase 2

grep -rn 'Verdict::pass.*stub' crates/ --include='*.rs' | grep -v test
# Should return 0 hits after Phase 2
```

---

## Sources

Implementation plan derived from:

- Gate pipeline audit (AUDIT.md in this directory)
- Issues catalog (ISSUES.md in this directory)
- Goals document (GOALS.md in this directory)
- `crates/roko-gate/src/` -- all 40 source files
- `crates/roko-gate/tests/` -- 4 integration test files
- `crates/roko-cli/src/orchestrate.rs` -- current gate integration
- `crates/roko-core/src/foundation.rs` -- GateRunner, GateConfig, GateReport
- `tmp/solutions/runner/LESSONS.md` -- anti-pattern lessons from batch runs
