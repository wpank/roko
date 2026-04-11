# Runner 13-16 — Gates + Providers + Cognitive + CLI/TUI

> **Give this entire file to a fresh agent.** Four cross-cutting plans that can be parallelized.

---

## Context

Codebase: `/Users/will/dev/nunchi/roko/roko`. These four plans address cross-cutting concerns:
- **13:** Unify 3 gate dispatch paths into one `GateService`
- **14:** Thread `ResolvedRuntimeConfig` everywhere, add stderr classifier, DOA detection, session resume
- **15:** Delete ~110K LOC of pheromones + daimon; add FailureTracker + WarningStore
- **16:** One chat loop, wire primitives, TUI on projection

**Read first:**

1. `tmp/workflow/implementation-plans/13-gate-pipeline-unification.md`
2. `tmp/workflow/implementation-plans/14-providers-action-plan.md`
3. `tmp/workflow/implementation-plans/15-cognitive-layer-cleanup.md`
4. `tmp/workflow/implementation-plans/16-cli-tui-rendering-convergence.md`

---

## Plan 13 — Gate Pipeline Unification

### 13-1: Align `GateService` rungs with `registry.rs::GATE_SPECS`

**File:** `crates/roko-gate/src/gate_service.rs`

Add `resolve_gate(name)` that looks up in `GATE_SPECS`. Dispatch by `GateKind`. Check `required_inputs` from `GateRunContext`.

### 13-2: Migrate ACP `run_gates`

Replace inline `CompileGate → TestGate → ClippyGate` in `crates/roko-acp/src/runner.rs` with `gate_runner.run_gates(GateConfig { ... })`.

### 13-3: Wire `LlmJudgeOracle` (per plan 01-E)

Rung 6 dispatches to `LlmJudgeOracle` via `GateRunContext.judge_oracle`.

### 13-4: `ServiceFactory` attaches adaptive thresholds

`ServiceFactory::build` calls `GateService::new().with_adaptive_thresholds(thresholds_handle)`.

### 13-5: Shared `Arc<Mutex<AdaptiveThresholds>>`

Same instance in `GateService` (reads) and `ThresholdSink` (writes).

---

## Plan 14 — Providers Action Plan

### 14-1: `ResolvedRuntimeConfig` everywhere

Define in `crates/roko-core/src/config/provenance.rs`. Thread from `main.rs` through every CLI command.

### 14-2: `detect_auth(config)` checks `default_backend` first

**File:** `crates/roko-cli/src/auth_detect.rs`

### 14-3: `--fallback-model` in Claude CLI spawns

**File:** `crates/roko-agent/src/provider/claude_cli/mod.rs` — add arg from config.

### 14-4: Stderr classifier

Create `crates/roko-agent/src/stderr_classifier.rs` with `classify(line) -> StderrLine { Benign | Important | Error }`.

### 14-5: DOA detection

Create `crates/roko-agent/src/spawn_wrapper.rs` with `spawn_with_doa_detection(invocation, 2s_threshold)`.

Classify: `BinaryMissing`, `AuthFailed`, `RateLimited`, `ModelNotAvailable`, `Unknown`.

### 14-6: Unified per-role tool policy

Create `policy_for_role(role, contract) -> RoleToolPolicy`. Delete `claude_tool_allowlist` and `resolve_tool_policy`.

### 14-7: Session resume in chat

Pass `session_id` via `routing_hints: ["claude:resume:<id>"]`. Adapter reads and adds `--resume <id>`.

### 14-8: `roko config doctor`

New command printing config provenance, auth detection, provider health, routing defaults.

---

## Plan 15 — Cognitive Layer Cleanup

### 15-1: Delete pheromones

Delete `crates/roko-orchestrator/src/coordination.rs` (~68K LOC). Replace all `PheromoneStore` / `active_pheromone_chunks` with `WarningStore::push(msg)`.

Create `crates/roko-runtime/src/warning_store.rs` with `push(String)`, `snapshot() -> Vec<String>`.

### 15-2: Delete `roko-daimon`

Delete entire `crates/roko-daimon/` crate. Remove from workspace `Cargo.toml`. Keep `NoOpAffectPolicy` in `roko-core`.

Create `FailureTracker` in `crates/roko-runtime/src/failure_tracker.rs`:

```rust
pub struct FailureTracker {
    consecutive_by_role: Mutex<HashMap<String, u32>>,
    last_kind: Mutex<HashMap<String, FailureKind>>,
}
impl FailureTracker {
    pub fn should_restrict_tools(&self, role: &str, threshold: u32) -> bool;
}
```

Create `FailureTrackerSink` in `crates/roko-learn/src/sinks/failure_tracker_sink.rs`.

### 15-3: Refactor distillation + dreams

Per plan 01-F, inject `Arc<dyn ModelCaller>`. Delete direct env reads.

### 15-4: Delete HDC

Delete `crates/roko-learn/src/hdc.rs`. Set `hdc_fingerprint: None` in new episodes.

---

## Plan 16 — CLI/TUI Rendering Convergence

### 16-1: `ResponseRenderer` trait

Create `crates/roko-cli/src/render/mod.rs`:

```rust
pub trait ResponseRenderer: Send {
    fn render_text(&mut self, text: &str);
    fn render_tool_call(&mut self, call: &ToolCallEvent);
    fn render_tool_output(&mut self, output: &ToolOutputEvent);
    fn render_cost(&mut self, summary: &CostSummary);
    fn render_gate_verdict(&mut self, verdict: &GateVerdict);
    fn render_error(&mut self, error: &str, suggestions: &[String]);
}
```

Implement `InlineRenderer` (ratatui), `PlainRenderer` (no styling).

### 16-2: Extract one chat loop

Create `crates/roko-cli/src/chat_session_loop.rs`:

```rust
pub trait ChatBackend: Send {
    async fn send_turn(&mut self, prompt: String) -> Result<DispatchResult>;
}
pub async fn run_chat_loop<R: ResponseRenderer, B: ChatBackend>(...) -> Result<()>;
```

`run_unified_inline` and `run_chat_inline` become 30-LOC wrappers.

### 16-3: Wire built primitives

`ToolCallBlock`, `CostWaterfall`, `DiffBlock`, `SessionSummary` into `InlineRenderer`.

### 16-4: TUI on projection

`tui/dashboard.rs` reads `RuntimeProjection::dashboard_view()` not disk files.

### 16-5: Delete `extract_clean_text`

(After all callers migrated.)

---

## Verification

```bash
# Gates unified
rg 'CompileGate|TestGate|ClippyGate' crates/roko-acp/ --type rust
# returns 0

# Pheromones gone
rg 'PheromoneStore' crates/ --type rust
# returns 0

# Daimon gone
ls crates/roko-daimon
# fails

# Chat loop extracted
wc -l crates/roko-cli/src/chat_inline.rs
# < 1500

# Config threaded
rg 'ResolvedRuntimeConfig' crates/roko-cli/src/ --type rust | wc -l
# ≥ 5

cargo test --workspace
```
