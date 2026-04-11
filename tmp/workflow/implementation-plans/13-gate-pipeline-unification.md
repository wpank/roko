# 13 — Gate Pipeline Unification

> Cross-cutting plan covering `tmp/workflow/11-gate-pipeline-audit.md`. Closes the "three gate dispatch paths" problem.

---

## Status (2026-05-01)

**PARTIAL.** `GateRunner` trait + `GateService` exist; ACP and orchestrate paths still bypass.

**What's done:**

- `roko_core::foundation::GateRunner` trait — `crates/roko-core/src/foundation.rs:408-413`
- `GateService` impl — `crates/roko-gate/src/gate_service.rs`
- `GateRegistry`, `GateSpec`, `GateKind`, `GATE_SPECS` — `crates/roko-gate/src/registry.rs`
- 7-rung canonical `run_canonical_rung` — `crates/roko-gate/src/rung_dispatch.rs`
- Adaptive thresholds — `crates/roko-gate/src/adaptive_threshold.rs`
- Gate feedback classifier — `crates/roko-gate/src/feedback.rs`
- Live gates: `compile`, `clippy`, `test`, `shell`
- `EffectDriver` calls `GateService` via `EffectServices.gate_runner`

**What's not:**

- **Three dispatch paths still coexist:**
  1. `GateService` (used by `EffectDriver` / `WorkflowEngine`)
  2. `rung_dispatch::run_canonical_rung` (used by `roko-cli/src/runner/gate_dispatch.rs`)
  3. ACP `run_gates` inline (Compile → Test → Clippy hardcoded) — `crates/roko-acp/src/runner.rs`
- **Rung index semantics differ:**
  - Canonical lib (`registry.rs`): rung 3 = Symbol; rung 4 = GeneratedTest+VerifyChain; rung 5 = PropertyTest+FactCheck; rung 6 = LlmJudge+Integration
  - `GateService` rung map: rung 3 = `diff:git`; rung 6 explicitly skipped ("not implemented")
- **Rungs 3–6 often return stubs** because inputs (symbol manifest, artifact store, oracles) aren't wired
- **LLM judge bypass:** `AgentJudgeOracle` (orchestrate.rs ~3093-3159, feature-gated) hardcodes `Command::new("claude")` + model
- **Adaptive thresholds:** ACP runner has its own EMA save logic; `ServiceFactory` uses `GateService::new()` (no `with_adaptive_thresholds`)
- **Gate feedback to retry prompt:** `EffectDriver::spawn_agent` always passes `gate_feedback: Vec::new()` (per plan 07 Step 1)
- **`feedback_for_agent` from `feedback.rs`** only used in orchestrate.rs (dead path)
- **6 standalone gates (DiffGate, CodeExecutionGate, BenchmarkRegressionGate, FormatCheckGate, SecurityScanGate)** built but no callers
- **3 composition wrappers (ParallelGate, VotingGate, FallbackGate)** test-only

---

## Goal

`GateService` is the **only** gate execution surface. Its `run_gates(GateConfig)` API supports the full 7-rung canonical pipeline. ACP and the legacy runner gate dispatcher both delegate to `GateService`. Gate feedback flows into the next agent prompt via `PromptSpec.gate_feedback`. LLM judge gate uses `ModelCallService`. Rung index semantics align with the `registry.rs` canonical mapping. Adaptive thresholds shared via one `Arc<Mutex<AdaptiveThresholds>>` instance.

---

## Why This Exists (Anti-Patterns Eliminated)

- **#1 Just Shell Out** — `AgentJudgeOracle` hardcodes Claude CLI
- **#3 Build Another Runtime** — three gate dispatchers for the same conceptual operation
- **#4 Features in Wrong Layer** — ACP runner has inline adaptive-skip heuristic
- **#6 Feedback Afterthought** — `feedback_for_agent` only fires from dead code
- **#7 Copy-Paste** — three separate `run_gates` implementations with different defaults

---

## Existing Code — Read These First

```rust
// crates/roko-core/src/foundation.rs
#[async_trait]
pub trait GateRunner: Send + Sync {
    async fn run_gates(&self, config: GateConfig) -> Result<GateReport>;
}

pub struct GateConfig {
    pub workdir: PathBuf,
    pub enabled_gates: Vec<String>,
    pub shell_gates: Vec<ShellGateCommand>,
    pub max_rung: Option<u8>,
}
```

```rust
// crates/roko-gate/src/gate_service.rs
pub struct GateService {
    adaptive: Option<Arc<Mutex<AdaptiveThresholds>>>,
}

impl GateService {
    pub const fn new() -> Self;
    pub fn with_adaptive_thresholds(mut self, thresholds: AdaptiveThresholds) -> Self;
}
```

```rust
// crates/roko-gate/src/registry.rs
pub static GATE_SPECS: &[GateSpec] = &[
    GateSpec { name: "compile", aliases: &["build", "cargo:build"], rung: 0, kind: GateKind::Compile, required_inputs: &[] },
    GateSpec { name: "clippy",  aliases: &["lint"],                rung: 1, kind: GateKind::Lint,    required_inputs: &[] },
    GateSpec { name: "test",    aliases: &["unit-test"],           rung: 2, kind: GateKind::Test,    required_inputs: &[] },
    GateSpec { name: "symbol",  aliases: &[],                       rung: 3, kind: GateKind::Symbol,  required_inputs: &["SymbolManifest"] },
    GateSpec { name: "generated_test", aliases: &[], rung: 4, kind: GateKind::GeneratedTest, required_inputs: &["ArtifactStore"] },
    GateSpec { name: "property_test", aliases: &[], rung: 5, kind: GateKind::PropertyTest, required_inputs: &["PerplexityOracle"] },
    GateSpec { name: "llm_judge", aliases: &[], rung: 6, kind: GateKind::LlmJudge, required_inputs: &["JudgeOracle"] },
];
```

---

## Implementation Steps

### Step 1 — Align `GateService` rungs with `registry.rs`

**File:** `crates/roko-gate/src/gate_service.rs`

Today `GateService` maps gates to rungs ad-hoc. Replace with lookup against `GATE_SPECS`:

```rust
impl GateService {
    fn resolve_gate(&self, name: &str) -> Option<&'static GateSpec> {
        GATE_SPECS.iter().find(|s| s.name == name || s.aliases.contains(&name))
    }

    async fn run_gate(&self, name: &str, config: &GateConfig, ctx: &GateRunContext) -> GateVerdict {
        let Some(spec) = self.resolve_gate(name) else {
            return GateVerdict::skipped(name, "unknown gate");
        };
        if let Some(max) = config.max_rung {
            if spec.rung > max { return GateVerdict::skipped(name, "above max_rung"); }
        }
        // dispatch by kind; required_inputs validated upfront
        if !ctx.has_inputs(spec.required_inputs) {
            return GateVerdict::skipped(name, "missing required inputs");
        }
        match spec.kind {
            GateKind::Compile => run_compile_gate(config, ctx).await,
            GateKind::Lint    => run_clippy_gate(config, ctx).await,
            GateKind::Test    => run_test_gate(config, ctx).await,
            GateKind::Shell   => run_shell_gate(config, ctx).await,
            GateKind::Symbol  => run_symbol_gate(config, ctx).await,
            GateKind::GeneratedTest => run_generated_test_gate(config, ctx).await,
            GateKind::PropertyTest  => run_property_test_gate(config, ctx).await,
            GateKind::FactCheck     => run_fact_check_gate(config, ctx).await,
            GateKind::LlmJudge      => run_llm_judge_gate(config, ctx).await,
            GateKind::Integration   => run_integration_gate(config, ctx).await,
        }
    }
}
```

`GateRunContext` carries optional inputs (`symbol_manifest`, `artifact_store`, `judge_oracle`, `perplexity_oracle`). Wired by `ServiceFactory` per plan dependencies.

### Step 2 — Migrate ACP runner to `GateService`

**File:** `crates/roko-acp/src/runner.rs`

Today `run_gates(...)` runs Compile → Test → Clippy inline with adaptive skip. Replace:

```rust
async fn run_gates(&self) -> Result<GateReport> {
    self.services.gate_runner.run_gates(GateConfig {
        workdir: self.workdir.clone(),
        enabled_gates: self.config.enabled_gates.clone(),    // ["compile", "test", "clippy"]
        shell_gates: vec![],
        max_rung: Some(2),
    }).await
}
```

The skip heuristic (EMA pass-rate > threshold for 20+ consecutive passes) is now handled by `GateService` via `with_adaptive_thresholds`. Delete the inline ACP version.

### Step 3 — Migrate `runner/gate_dispatch.rs` to `GateService`

**File:** `crates/roko-cli/src/runner/gate_dispatch.rs`

Today `run_canonical_rung(rung, inputs, config)` is called from `runner/event_loop.rs`. Once event_loop is gone (plan 12 § 5), this whole file goes away too. Until then:

- Make `gate_dispatch::run_rung_via_service(rung, ctx)` a thin wrapper that builds `GateConfig` + calls `GateService`
- `runner/event_loop.rs` callers switch to `run_rung_via_service`

### Step 4 — Rewrite LLM judge to use `ModelCallService`

**File:** Move `AgentJudgeOracle` from `crates/roko-cli/src/orchestrate.rs:~3093-3159` into `crates/roko-gate/src/llm_judge_oracle.rs`:

```rust
pub struct LlmJudgeOracle {
    service: Arc<dyn ModelCaller>,
    judge_model: Option<String>,           // None → router decides
    judge_role: String,                    // "judge" — registered in core_roles.toml
}

#[async_trait]
impl JudgeOracle for LlmJudgeOracle {
    async fn judge(&self, prompt: &str, context: &JudgeContext) -> JudgeVerdict {
        let req = ModelCallRequest {
            model: self.judge_model.clone().unwrap_or_default(),
            system: Some(judge_system_prompt(&self.judge_role)),
            messages: vec![
                ChatMessage { role: MessageRole::System, content: judge_system_prompt(&self.judge_role) },
                ChatMessage { role: MessageRole::User, content: build_judge_prompt(prompt, context) },
            ],
            role: Some(self.judge_role.clone()),
            caller: Some("gate.judge".into()),
            cache_policy: CachePolicy::Bypass,             // judges should not be cached
            ..Default::default()
        };
        match self.service.call(req).await {
            Ok(resp) => parse_judge_verdict(&resp.content),
            Err(e) => JudgeVerdict::Error(e.to_string()),
        }
    }
}
```

Wire `ServiceFactory` so `GateRunContext.judge_oracle = Some(Arc::new(LlmJudgeOracle::new(model_caller, ...)))`.

Then `GateService::run_llm_judge_gate` consumes the oracle. After this:

- Delete `AgentJudgeOracle` from `orchestrate.rs`
- The judge call is recorded as an episode (caller `gate.judge`)

### Step 5 — Wire `feedback_for_agent` into retry prompts

**File:** `crates/roko-runtime/src/effect_driver.rs` (per plan 07 § Step 1)

After a gate fails, the FSM emits `SpawnImplementer { gate_feedback: Vec<GateFeedback>, ... }`. Build the `Vec<GateFeedback>` from `GateReport`:

```rust
fn build_gate_feedback(report: &GateReport) -> Vec<GateFeedback> {
    report.verdicts.iter()
        .filter(|v| !v.passed)
        .map(|v| {
            let classified = roko_gate::feedback::feedback_for_agent(&v.output);
            GateFeedback {
                gate_name: v.gate_name.clone(),
                rung: rung_for_gate(&v.gate_name),
                passed: false,
                errors: classified.errors,
                warnings: classified.warnings,
                suggestions: classified.suggestions,
            }
        })
        .collect()
}
```

The FSM stores the most recent `Vec<GateFeedback>` in its state; passes it to the next `SpawnImplementer` action.

### Step 6 — Adaptive thresholds: one shared `Arc<Mutex<...>>`

**File:** `crates/roko-orchestrator/src/service_factory.rs`

Today `ServiceFactory::build` calls `GateService::new()` — no adaptive thresholds attached. Fix:

```rust
let thresholds = AdaptiveThresholds::load_or_default(&workdir.join(".roko/learn/gate-thresholds.json"))?;
let thresholds_handle = Arc::new(Mutex::new(thresholds));
let gate_runner = Arc::new(GateService::new().with_adaptive_thresholds(thresholds_handle.clone()));
let threshold_sink = Arc::new(ThresholdSink::new(thresholds_handle.clone(), workdir.join(".roko/learn/gate-thresholds.json")));
let feedback_sink = MultiSink::new(vec![..., threshold_sink, ...]);
```

Same `thresholds_handle` lives in both `GateService` (reads) and `ThresholdSink` (writes). After plan 03, the sink updates EMA on every `GateResult` event; `GateService` re-checks the EMA on every rung selection.

### Step 7 — Wire 6 standalone gates (or delete)

For each of `DiffGate`, `CodeExecutionGate`, `BenchmarkRegressionGate`, `FormatCheckGate`, `SecurityScanGate`:

1. Add a `GateSpec` entry in `registry.rs` (e.g. `format_check` rung 1)
2. Add the dispatch arm in `GateService::run_gate`
3. If still no caller after 1 release, delete

`DiffGate` is genuinely useful (compares pre/post artifact diffs). `BenchmarkRegressionGate` is useful for plans that touch performance-sensitive code. `FormatCheckGate` is rust-fmt; `compile` already covers most use-cases but `format_check` is faster. `SecurityScanGate` integrates `cargo audit` or `gitleaks` — useful for CI plans.

Delete `CodeExecutionGate` if it's literally the same as `shell` gate.

### Step 8 — Tests for full canonical pipeline

```rust
#[tokio::test]
async fn full_7_rung_pipeline_with_all_inputs() {
    let temp = test_workdir_with_symbol_manifest_and_oracles();
    let services = ServiceFactory::for_test(temp.path()).await?;
    let report = services.gate_runner().run_gates(GateConfig {
        workdir: temp.path().into(),
        enabled_gates: vec![
            "compile", "clippy", "test", "symbol",
            "generated_test", "property_test", "llm_judge"
        ],
        shell_gates: vec![],
        max_rung: Some(6),
    }).await?;

    assert_eq!(report.verdicts.len(), 7);
    for v in &report.verdicts {
        assert!(v.passed || v.skipped, "gate {} failed: {}", v.gate_name, v.output);
    }
}

#[tokio::test]
async fn llm_judge_routes_through_model_call_service() {
    let recorded = Arc::new(Mutex::new(Vec::new()));
    let service = Arc::new(test_model_caller_recording(recorded.clone()));
    let oracle = LlmJudgeOracle::new(service, None, "judge".into());
    let _ = oracle.judge("does X work?", &test_context()).await;

    let calls = recorded.lock().await;
    assert_eq!(calls.len(), 1);
    assert_eq!(calls[0].caller.as_deref(), Some("gate.judge"));
    assert_eq!(calls[0].cache_policy, CachePolicy::Bypass);
}

#[tokio::test]
async fn gate_failure_propagates_into_retry_prompt() {
    let services = test_services_with_failing_gate();
    let engine = WorkflowEngine::new(services);
    let report = engine.run(test_run_config()).await?;
    let assembled_prompts = engine.captured_prompts();
    assert!(assembled_prompts.iter().any(|p| p.system.contains("gate failure: compile")));
    assert!(assembled_prompts.iter().any(|p| p.system.contains("error[")));
}
```

---

## Anti-Patterns To Avoid

| ID | Manifestation | How to avoid |
|---|---|---|
| #1 Just shell out | New gates spawning `claude` directly | LLM judge uses `ModelCallService` |
| #3 Build another runtime | Adding a `FastGateRunner` for HTTP that bypasses adaptive thresholds | One `GateService` |
| #7 Copy-paste | Each entry point implementing skip heuristic | One `AdaptiveThresholds` shared via `Arc<Mutex<...>>` |
| #4 Wrong layer | Adding skip-rung logic in ACP runner | All skip logic in `GateService` |

---

## Things NOT To Do

1. **Don't call `cargo build` directly from `EffectDriver`.** Always go through `GateService`. The driver passes `enabled_gates: vec!["compile"]`; the service knows how.
2. **Don't make `GateService::run_gates` synchronous.** Some gates (LLM judge, fact check) are inherently async.
3. **Don't preserve duplicate stub returns.** If a rung's required input is missing, return `GateVerdict::skipped` with a clear reason — don't fall through to a stub success.
4. **Don't make adaptive threshold writes blocking.** They're fire-and-forget after `flush()` — sink writes asynchronously.
5. **Don't reuse `gate_dispatch.rs` once event_loop is gone.** Delete in plan 12 § 5.
6. **Don't add gate priority overrides per role.** Roles are already filtered by `tools.capabilities`. Adding gate-level filtering muddles concerns.
7. **Don't run the LLM judge by default.** It's expensive. Default `max_rung: 2` for chat; `max_rung: 5` for plan run; `max_rung: 6` only when explicitly requested.

---

## Tests / Proof Criteria

```bash
# 1. Single dispatch surface
rg 'fn run_gate|fn run_gates|fn run_gate_rung|fn run_canonical_rung' crates/ --type rust | grep -v test
# expected: only inside GateService and registry's helpers

# 2. LLM judge through ModelCallService
rg 'AgentJudgeOracle|claude-sonnet' crates/roko-gate/ crates/roko-cli/src/orchestrate.rs --type rust
# expected: 0 (post-migration)

# 3. Gate feedback wired into retry
rg 'gate_feedback' crates/roko-runtime/src/effect_driver.rs
# expected: build_gate_feedback function + non-empty Vec passed to SpawnImplementer

# 4. ACP runner doesn't have inline gate code
rg 'CompileGate|TestGate|ClippyGate' crates/roko-acp/ --type rust
# expected: 0 direct usages
```

Functional proofs:

- [ ] All 3 unit tests above pass
- [ ] `roko run "fix bug"` after a `cargo build` failure shows the error in the next prompt's Layer 4b
- [ ] `roko plan run` with `max_rung: 5` runs property_test and fact_check
- [ ] LLM judge gate's call appears in `.roko/episodes.jsonl` with `caller: "gate.judge"`
- [ ] After 20 consecutive `cargo test` passes, the next plan run skips the test gate (adaptive threshold check)
- [ ] ACP runner produces same `RuntimeEvent` sequence for gates as `roko run` does

---

## Dependencies

- **Plan 01 (ModelCallService)** — for LLM judge
- **Plan 02 (PromptAssembly)** — `gate_feedback` slot in `PromptSpec`
- **Plan 03 (FeedbackService)** — `ThresholdSink` shared with `GateService`
- **Plan 07 (EffectDriver)** — `build_gate_feedback` and propagation
- **Plan 08 (CascadeRouter)** — judge routes through router

Can start in parallel with these once `GateRunContext` shape is decided.

---

## Estimated Effort

**M.** ~1-1.5 weeks.

- Step 1 (rung alignment) — M (2 days)
- Step 2 (ACP migration) — S (1 day)
- Step 3 (runner gate_dispatch) — S (1 day)
- Step 4 (LLM judge migration) — M (2 days)
- Step 5 (gate feedback wiring) — S (1 day; mostly plan 07 work)
- Step 6 (adaptive thresholds shared) — S (1 day)
- Step 7 (6 standalone gates) — M (2 days; either wire or delete)
- Step 8 (tests) — S (1 day)
