# AUDIT: Batch R2_C01 — Document current gate data flow

Run: run-20260429-030528 | Phase: AUDIT | Model: codex-5.5

## Your Role

You are an **auditor**. A fast implementation agent just completed batch `R2_C01`.
Your job is to verify correctness and fix any issues — do NOT rewrite from scratch.

## Audit Checklist

1. **Compiles:** `cargo check -p <crate>` for each crate touched by this batch
2. **Clippy clean:** `cargo clippy -p <crate> --no-deps -- -D warnings`
3. **Prompt compliance:** Compare the implementation against the original prompt below
4. **No regressions:** Changed files don't break existing functionality
5. **Anti-patterns:** No stubs that silently pass, no inline prompts, no raw CLI spawns
6. **Correct types:** Field names, method signatures, and imports match the actual codebase
7. **Tests pass:** If the prompt required tests, verify they pass

## If You Find Issues

Fix them directly in the files. Then run the verification commands from the prompt.
If you cannot fix an issue, leave a comment in the file explaining why.

## Scope

Only touch files in the batch's write scope. Do NOT refactor unrelated code.

---

## Original Implementation Prompt

## Task

Document current gate data flow

## Runner Context

You are working in runner `mega-parity`, batch R2_C01.
This batch is part of Runner 2: execution-contract — Make CLI execution contracts truthful enough that demo scenarios and agent sessions can rely on them.

## Problem

The gate pipeline has multiple layers (GateConfig in config.rs, gate_service.rs dispatch, orchestrate.rs invocation, individual gate implementations) and the data flow between them is undocumented. Without understanding how config flows to execution, subsequent batches risk breaking the pipeline or duplicating logic.

## Architecture Contract

This is a context-only batch. No code changes. Produce a reference document that C02-C06 depend on.
Target: gate verdicts distinguish pass/fail/skipped.

## What You Will Find In The Codebase

The following facts are confirmed from the actual source. Use them as your starting point.

### 1. Two separate `GateConfig` types exist — they are NOT the same type

**Type A: CLI config (TOML-parsed)**
File: `/Users/will/dev/nunchi/roko/roko/crates/roko-cli/src/config.rs`
Lines: 851–897
```rust
/// One gate entry in `roko.toml`. Multiple gates run in declaration order.
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum GateConfig {
    Shell {
        program: String,
        #[serde(default)]
        args: Vec<String>,
        #[serde(default = "default_gate_timeout")]
        timeout_ms: u64,
    },
    Compile {
        #[serde(default = "default_build_system")]
        build_system: String,
        #[serde(default = "default_gate_timeout_long")]
        timeout_ms: u64,
    },
    Clippy {
        #[serde(default = "default_build_system")]
        build_system: String,
        #[serde(default = "default_gate_timeout_long")]
        timeout_ms: u64,
    },
    Test {
        #[serde(default = "default_build_system")]
        build_system: String,
        #[serde(default = "default_gate_timeout_long")]
        timeout_ms: u64,
    },
}
```
TOML syntax: `[[gates]]\nkind = "shell"\nprogram = "cargo"\nargs = ["check"]`

**Type B: Foundation trait config (passed to GateRunner trait)**
File: `/Users/will/dev/nunchi/roko/roko/crates/roko-core/src/foundation.rs`
Lines: 207–216
```rust
pub struct GateConfig {
    pub workdir: PathBuf,
    pub enabled_gates: Vec<String>,   // gate names as strings
    pub max_rung: Option<u8>,
}
```
This type is used by `GateService` in `gate_service.rs`. It does NOT contain program/args.

### 2. `GateVerdict` type (foundation.rs, lines 219–225)

```rust
#[derive(Debug, Clone)]
pub struct GateVerdict {
    pub gate_name: String,
    pub passed: bool,
    pub output: String,
    pub duration_ms: u64,
}
```
Note: does NOT derive `Serialize` or `Deserialize`. Does NOT have `skipped` field.

### 3. `GateService::gate_for_name()` dispatch table

File: `/Users/will/dev/nunchi/roko/roko/crates/roko-gate/src/gate_service.rs`
Lines: 64–81
```rust
fn gate_for_name(&self, name: &str, build_system: BuildSystem) -> Option<Box<dyn Verify>> {
    match name {
        "compile" | "compile:cargo" => Some(Box::new(CompileGate::new(build_system))),
        "clippy" | "clippy:cargo"   => Some(Box::new(ClippyGate::new(build_system))),
        "test" | "test:cargo"       => Some(Box::new(TestGate::new(build_system))),
        "diff" | "diff:git"         => Some(Box::new(
            ShellGate::new("git", vec!["diff".into(), "--stat".into()]).with_timeout_ms(30_000),
        )),
        "fmt" | "fmt:cargo" | "format" => Some(Box::new(FormatCheckGate::cargo())),
        "custom" | "custom:shell" => {
            // TODO(converge): read custom command from GateConfig once it supports a custom_command field.
            Some(Box::new(ShellGate::new("true", vec![])))
        }
        "judge" | "llm-judge" => Some(Box::new(StubJudgeGate)),
        _ => None,                    // <-- "shell" is NOT recognized here
    }
}
```
**Critical gap**: `"shell"` is not in this match. `GateConfig::Shell` in roko.toml becomes the
string `"shell"` at line 7522 of orchestrate.rs, but `gate_for_name("shell")` returns `None`,
which causes a `passed: false` verdict with `output: "Unknown gate: shell"`.

### 4. `GateService::rung_for_name()` dispatch table

File: `/Users/will/dev/nunchi/roko/roko/crates/roko-gate/src/gate_service.rs`
Lines: 50–61
```rust
fn rung_for_name(name: &str) -> Option<u8> {
    match name {
        "compile" | "compile:cargo" => Some(0),
        "clippy" | "clippy:cargo"   => Some(1),
        "test" | "test:cargo"       => Some(2),
        "diff" | "diff:git"         => Some(3),
        "fmt" | "fmt:cargo" | "format" => Some(4),
        "custom" | "custom:shell"   => Some(5),
        "judge" | "llm-judge"       => Some(6),
        _ => None,                    // <-- "shell" returns None, so rung = u8::MAX
    }
}
```

### 5. What happens when `gate_for_name()` returns None

File: `/Users/will/dev/nunchi/roko/roko/crates/roko-gate/src/gate_service.rs`
Lines: 227–235
```rust
let Some(gate) = self.gate_for_name(&gate_name, build_system) else {
    verdicts.push(GateVerdict {
        gate_name: gate_name.clone(),
        passed: false,              // <-- treated as a FAILURE, not a skip
        output: format!("Unknown gate: {gate_name}"),
        duration_ms: 0,
    });
    break;                          // <-- stops all further gate execution
};
```

### 6. Adaptive-skip path (NOT the unknown-gate path)

File: `/Users/will/dev/nunchi/roko/roko/crates/roko-gate/src/gate_service.rs`
Lines: 215–225
```rust
if let Some(r) = rung {
    if self.should_skip_rung_adaptively(Some(r))? {
        verdicts.push(GateVerdict {
            gate_name: gate_name.clone(),
            passed: true,           // <-- adaptive skip counts as PASS (also wrong)
            output: format!("Skipped (adaptive: high pass rate for rung {r})"),
            duration_ms: 0,
        });
        continue;
    }
}
```

### 7. `ShellGate` struct (confirmed to exist)

File: `/Users/will/dev/nunchi/roko/roko/crates/roko-gate/src/shell.rs`
Lines: 21–55
```rust
pub struct ShellGate {
    program: String,
    args: Vec<String>,
    timeout_ms: u64,
    name: String,
}

impl ShellGate {
    pub fn new(program: impl Into<String>, args: Vec<String>) -> Self { ... }
    pub const fn with_timeout_ms(mut self, ms: u64) -> Self { ... }
    pub fn with_name(mut self, name: impl Into<String>) -> Self { ... }
}
```
Behavior: spawns the process, reads stdout+stderr, passes on exit code 0.
Used by `gate_for_name()` for `"diff"` and `"custom"` arms already.

### 8. `StubJudgeGate` (exists, deliberately fails)

File: `/Users/will/dev/nunchi/roko/roko/crates/roko-gate/src/gate_service.rs`
Lines: 163–192
```rust
struct StubJudgeGate;
impl Verify for StubJudgeGate {
    async fn verify(&self, _signal: &Engram, _ctx: &Context) -> Verdict {
        Verdict::fail("stub-llm-judge",
            "LLM judge gate not yet implemented — enable a real judge or remove from enabled_gates")
    }
}
```
This gate reports `passed: false`. It is NOT skipped, it actively fails.

### 9. Orchestrate.rs: how CLI GateConfig becomes a string name

File: `/Users/will/dev/nunchi/roko/roko/crates/roko-cli/src/orchestrate.rs`
Lines: 7512–7524 (inside `run_with_v2_engine`)
```rust
let enabled_gates: Vec<String> = self
    .config
    .gates
    .iter()
    .map(|gate| match gate {
        crate::config::GateConfig::Compile { .. } => "compile".to_string(),
        crate::config::GateConfig::Clippy { .. }  => "clippy".to_string(),
        crate::config::GateConfig::Test { .. }    => "test".to_string(),
        crate::config::GateConfig::Shell { .. }   => "shell".to_string(),  // program/args discarded
    })
    .collect();
```
The `program` and `args` from `GateConfig::Shell` are silently discarded here. Only the string
`"shell"` is passed forward. Since `gate_for_name("shell")` returns None, the gate always fails.

### 10. run.rs: the correct pattern (for reference)

File: `/Users/will/dev/nunchi/roko/roko/crates/roko-cli/src/run.rs`
Lines: 2146–2157 (inside `run_gate()`)
```rust
async fn run_gate(cfg: &GateConfig, input: &Engram, ctx: &Context) -> Verdict {
    match cfg {
        GateConfig::Shell { program, args, timeout_ms } => {
            ShellGate::new(program, args.clone())
                .with_timeout_ms(*timeout_ms)
                .verify(input, ctx)
                .await
        }
        // ...
    }
}
```
This is the correct pattern. `run.rs` directly dispatches on the typed `GateConfig` enum, so
program/args are available. The `GateService` path loses them because it only receives a `Vec<String>`.

### 11. Second `GateVerdict` type in roko-learn (different struct)

File: `/Users/will/dev/nunchi/roko/roko/crates/roko-learn/src/episode_logger.rs`
Lines: 88–100
```rust
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct GateVerdict {
    #[serde(default)]
    pub gate: String,
    #[serde(default)]
    pub passed: bool,
    #[serde(default)]
    pub signature: Option<String>,
}
```
This is a DIFFERENT type from `roko_core::foundation::GateVerdict`. It is used in `Episode`
records for the learning subsystem. It already derives Serialize/Deserialize. It also does NOT
have a `skipped` field yet.

### 12. Adaptive threshold observation in orchestrate.rs

File: `/Users/will/dev/nunchi/roko/roko/crates/roko-cli/src/orchestrate.rs`
Line: 16174–16177
```rust
for recorded in &recorded_verdicts {
    self.adaptive_thresholds
        .observe(recorded.rung.as_index(), recorded.verdict.passed);
}
```
This feeds every verdict (including future skipped ones) into the EMA. After C04 adds `skipped`,
C05 must filter skipped verdicts out of this observe call.

## Changes Required

This is a context-only batch. Write the document at:
`tmp/runners/mega-parity/context/R2_C01_gate_dataflow.md`

The document must cover:
1. The two distinct `GateConfig` types and where each is used
2. The `gate_for_name()` dispatch table with the gap for `"shell"`
3. The `rung_for_name()` dispatch table with the gap for `"shell"`
4. How program/args are lost in the v2 engine path
5. The `ShellGate` constructor signature
6. `GateVerdict` in foundation.rs vs `GateVerdict` in episode_logger.rs
7. Where adaptive thresholds observe gate verdicts (line 16176)
8. The `StubJudgeGate` behavior

## Write Scope (files you may modify)

- `tmp/runners/mega-parity/context/R2_C01_gate_dataflow.md` (create this file)

## Read-Only Context (do not modify these)

- `crates/roko-core/src/foundation.rs`
- `crates/roko-gate/src/gate_service.rs`
- `crates/roko-cli/src/orchestrate.rs`
- `crates/roko-gate/src/shell.rs`
- `crates/roko-cli/src/config.rs`
- `crates/roko-cli/src/run.rs`

## Acceptance Criteria

- [ ] GateConfig::Shell data flow is mapped from TOML to execution
- [ ] gate_for_name() dispatch table is documented with the "shell" gap identified
- [ ] ShellGate existence is confirmed with exact file:line reference
- [ ] GateVerdict fields in both foundation.rs and episode_logger.rs are documented
- [ ] orchestrate.rs gate invocation flow and the adaptive threshold observe call are mapped
- [ ] No source code was modified

## Verification

N/A (context-only batch)

## Do NOT

- Change any source code
- Guess at implementation — read the actual gate code
- Document aspirational behavior

## Evidence

COMPREHENSIVE-ISSUES 2.x

---

## Read-Only Context (do not modify)

### `crates/roko-core/src/foundation.rs` (509 lines — signatures only)

```rust
20:pub struct ModelCallRequest {
23:    pub model: String,
26:    pub system: Option<String>,
29:    pub messages: Vec<ChatMessage>,
32:    pub max_tokens: Option<u32>,
35:    pub temperature: Option<f32>,
38:    pub role: Option<String>,
41:    pub caller: Option<String>,
44:    pub run_id: Option<String>,
47:    pub prompt_section_ids: Vec<String>,
50:    pub knowledge_ids: Vec<String>,
53:    pub budget: Option<TokenBudget>,
56:    pub budget_remaining: Option<f64>,
59:    pub routing_hints: Vec<String>,
62:    pub cache_policy: CachePolicy,
70:pub mod caller {
71:    pub const CLI: &str = "cli";
72:    pub const SERVE: &str = "serve";
73:    pub const RESEARCH: &str = "research";
74:    pub const DREAMS: &str = "dreams";
79:pub enum CachePolicy {
91:pub struct TokenBudget {
93:    pub max_input: Option<u64>,
95:    pub max_output: Option<u64>,
97:    pub max_cost_usd: Option<f64>,
102:pub enum GatewayError {
117:impl From<GatewayError> for RokoError {
126:pub struct ChatMessage {
127:    pub role: MessageRole,
128:    pub content: String,
133:pub enum MessageRole {
141:pub struct ModelCallResponse {
142:    pub content: String,
143:    pub model: String,
144:    pub usage: TokenUsage,
145:    pub stop_reason: Option<String>,
147:    pub request_id: Option<String>,
152:pub struct TokenUsage {
153:    pub input_tokens: u64,
154:    pub output_tokens: u64,
```

### `crates/roko-gate/src/gate_service.rs` (667 lines — signatures only)

```rust
26:pub struct GateService {
30:impl GateService {
33:    pub const fn new() -> Self {
45:    pub fn with_adaptive_thresholds(mut self, thresholds: AdaptiveThresholds) -> Self {
137:fn skipped_gate_verdict(
153:struct FormatCheckGate {
157:impl FormatCheckGate {
166:impl roko_core::Cell for FormatCheckGate {
181:impl Verify for FormatCheckGate {
195:struct StubJudgeGate;
197:impl roko_core::Cell for StubJudgeGate {
212:impl Verify for StubJudgeGate {
225:impl Default for GateService {
232:impl GateRunner for GateService {
365:fn to_gate_verdict(gate_name: String, verdict: Verdict) -> GateVerdict {
```

### `crates/roko-cli/src/orchestrate.rs` (22076 lines — signatures only)

```rust
231:fn domain_uses_git(domain: &TaskDomain) -> bool {
235:fn workflow_enabled_gate_names(gates: &[crate::config::GateConfig]) -> Vec<String> {
248:fn workflow_shell_gate_commands(gates: &[crate::config::GateConfig]) -> Vec<CoreShellGateCommand> {
267:fn resolve_task_role(role_str: Option<&str>) -> AgentRole {
277:fn model_experiments_path(workdir: &Path) -> PathBuf {
284:fn failure_pattern_store_path(workdir: &Path) -> PathBuf {
291:fn pre_agent_remediation_log_path(workdir: &Path) -> PathBuf {
298:fn daimon_state_path(workdir: &Path) -> PathBuf {
302:fn latency_registry_path(workdir: &Path) -> PathBuf {
313:fn routing_log_path(workdir: &Path) -> PathBuf {
319:fn custody_logger_for(workdir: &Path) -> CustodyLogger {
323:fn cfactor_history_path(workdir: &Path) -> PathBuf {
330:struct HeartbeatCounts {
341:struct SectionEffectCatalystSource {
346:impl CatalystSignalSource for SectionEffectCatalystSource {
372:struct StaticCFactorSource {
376:impl CFactorSource for StaticCFactorSource {
443:fn predictive_policy_sections(
475:fn predictive_calibration_summary_section(
503:fn cfactor_policy_sections(source: Arc<dyn CFactorSource>) -> Vec<PromptSection> {
524:fn parse_count_tag(signal: &Engram, key: &str) -> usize {
531:fn top_cfactor_contributors(snapshot: &CFactor) -> (Vec<String>, Vec<String>) {
581:fn task_requirements_for_routing(
645:fn conductor_policy_path(workdir: &Path) -> PathBuf {
649:fn scrub_json_value(value: &serde_json::Value, policy: &ScrubPolicy) -> serde_json::Value {
668:fn scrub_body(body: &Body, policy: &ScrubPolicy) -> Body {
676:fn scrub_signal(signal: &Engram, policy: &ScrubPolicy) -> Engram {
688:fn scrub_agent_result(result: &AgentResult, policy: &ScrubPolicy) -> AgentResult {
702:fn state_dir(workdir: &Path) -> PathBuf {
706:fn executor_snapshot_path(workdir: &Path) -> PathBuf {
710:fn agent_invocation_ledger_path(workdir: &Path) -> PathBuf {
714:fn append_agent_invocation_record(workdir: &Path, record: &AgentInvocationSession) {
745:fn invocation_state_from_agent_result(result: &AgentResult) -> InvocationState {
763:pub fn save_snapshot_atomic(snapshot: &ExecutorSnapshot, path: &Path) -> Result<()> {
785:fn persisted_circuit_breaker_state(state: CircuitBreakerState) -> PersistedCircuitBreakerState {
805:fn restored_circuit_breaker_state(state: PersistedCircuitBreakerState) -> CircuitBreakerState {
848:fn sync_file_if_present(path: &Path) -> Result<()> {
858:fn load_roko_config(workdir: &Path) -> Result<RokoConfig> {
869:fn frequency_label(frequency: OperatingFrequency) -> &'static str {
877:fn task_runner_cost_table(resolved: &roko_core::agent::ResolvedModel) -> RunnerCostTable {
```

---

## Verification Commands

Run these and fix any failures:
```bash
cargo check --workspace
cargo clippy --workspace --no-deps -- -D warnings
```

## Do NOT

- Rewrite the entire implementation from scratch
- Add features not in the original prompt
- Modify files outside the write scope
- Skip running verification commands
