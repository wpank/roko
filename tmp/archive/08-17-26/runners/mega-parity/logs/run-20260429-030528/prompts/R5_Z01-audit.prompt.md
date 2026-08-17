# AUDIT: Batch R5_Z01 — Audit telemetry data flow

Run: run-20260429-030528 | Phase: AUDIT | Model: codex-5.5

## Your Role

You are an **auditor**. A fast implementation agent just completed batch `R5_Z01`.
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
Audit telemetry data flow — read-only, write findings to `.roko/GAPS.md`

## Runner Context
You are working in runner `mega-parity`, batch R5_Z01.
This batch is part of Runner 5: telemetry-learning — Make cost, usage, episodes, learning, and cascade router feedback truthful enough that dashboards show real data and self-improvement actually works.

## Problem
Telemetry data flows through multiple writers and readers with no single map. Unknown models appear as strings, costs show $0.00 when they should show "unknown", and dashboards may read paths that execution never writes to.

This is a **read-only audit batch**. No code changes. Read the files below and write your findings to `.roko/GAPS.md`.

## Step-by-step instructions

### Step 1: Read the stream-json parser

File: `crates/roko-agent/src/provider/claude_cli/stream.rs`

Key structs (verified line numbers):
- `ClaudeResultEvent` — lines 92–108
- `ClaudeUsage` — lines 112–121

`ClaudeResultEvent` fields:
```rust
pub session_id: String,           // serde(default) → ""
pub total_cost_usd: Option<f64>,  // serde(default) → None
pub num_turns: Option<u32>,       // serde(default) → None
pub is_error: bool,               // serde(default) → false
pub duration_ms: Option<f64>,     // serde(default) → None
pub duration_api_ms: Option<f64>, // serde(default) → None
pub usage: Option<ClaudeUsage>,   // serde(default) → None
```

`ClaudeUsage` fields (all `#[serde(default)]` on `u64` → 0 when absent):
```rust
pub input_tokens: u64,
pub output_tokens: u64,
pub cache_creation_input_tokens: u64,
pub cache_read_input_tokens: u64,
```

Verify: `grep -n "struct ClaudeResultEvent\|struct ClaudeUsage" crates/roko-agent/src/provider/claude_cli/stream.rs`

The model name is in `ClaudeSystemEvent.model` (the `system` init event), NOT in the result event.

### Step 2: Read `AgentResult.usage` shape

File: `crates/roko-agent/src/usage.rs`

```rust
// Line 1-4: Just a re-export
pub use roko_core::chat_types::Usage;
```

File: `crates/roko-core/src/foundation.rs` (or `crates/roko-core/src/chat_types.rs`)

Run: `grep -n "struct Usage" crates/roko-core/src/ -r`

The `Usage` struct has fields `input_tokens: u32`, `output_tokens: u32`, `cache_read_tokens: u32`, `cache_create_tokens: u32`, `cost_usd: f32`, `wall_ms: u64`. All non-Option.

File: `crates/roko-agent/src/agent.rs` lines 9–23

`AgentResult`:
```rust
pub struct AgentResult {
    pub output: Engram,
    pub trace: Vec<Engram>,
    pub usage: Usage,
    pub success: bool,
}
```

### Step 3: Read the bug — usage never extracted in claude_cli_agent.rs

File: `crates/roko-agent/src/claude_cli_agent.rs`, lines 656–664

```rust
AgentResult::ok(output_signal)
    .with_trace(self.stderr_trace(&stderr))
    .with_usage(Usage {
        wall_ms,
        ..Default::default()  // input_tokens=0, output_tokens=0, cost_usd=0.0
    })
```

Verify: `grep -n "with_usage\|Default::default" crates/roko-agent/src/claude_cli_agent.rs`

The `output_text()` function at line 361 extracts text from `parse_stream_events()`, but `total_cost_usd` and `usage` from the result event are never extracted into `AgentResult.usage`. All usage fields remain zero.

### Step 4: Read the efficiency event writer

File: `crates/roko-cli/src/orchestrate.rs`

Run: `grep -n "emit_efficiency_event\|AgentEfficiencyEvent" crates/roko-cli/src/orchestrate.rs | head -15`

Key lines:
- Line 17332–17385: `emit_efficiency_event()` builds `AgentEfficiencyEvent`
- Line 17339: `input_tokens: u64::from(result.usage.input_tokens)` — always 0
- Line 17340: `output_tokens: u64::from(result.usage.output_tokens)` — always 0
- Line 17344: `cost_usd: f64::from(result.usage.cost_usd)` — always 0.0
- Lines 17445–17503: `emit_failure_efficiency_event()` hardcodes zeros

Run: `grep -n "input_tokens: 0\|output_tokens: 0\|cost_usd: 0.0" crates/roko-cli/src/orchestrate.rs | head -10`

### Step 5: Read AgentEfficiencyEvent shape

File: `crates/roko-learn/src/efficiency.rs` (or search: `grep -rn "struct AgentEfficiencyEvent" crates/`)

All fields are non-Option: `model: String`, `input_tokens: u64`, `output_tokens: u64`, `cost_usd: f64`. Zero is used for "unknown."

### Step 6: Read episode write sites

File: `crates/roko-cli/src/orchestrate.rs`

Run: `grep -n "ep\.usage = Usage\|ep\.model" crates/roko-cli/src/orchestrate.rs | head -20`

Key lines:
- Line 10497–10503: Success episode — `cost_usd: f64::from(result.usage.cost_usd)` — always 0
- Line 12611–12625: Failure episode — same pattern
- Line 9691–9692: `if ep.model.trim().is_empty() { ep.model = model.to_string(); }`

The episode logger `Usage` struct is different from `roko_core::chat_types::Usage`:
Run: `grep -n "struct Usage" crates/roko-learn/src/episode_logger.rs`
Expected: `u64` fields instead of `u32`, `f64` instead of `f32`.

### Step 7: Find all "unknown-model" occurrences

Run:
```bash
grep -rn 'unknown-model' crates/ --include='*.rs' | grep -v target/
```

Document every file:line that uses `"unknown-model"` as a sentinel.

### Step 8: Read the CLI learn display

File: `crates/roko-cli/src/commands/learn.rs`, lines 226–355

Key reads:
- Line 240: `let total_cost: f64 = events.iter().map(|e| e.cost_usd).sum();` — sums zeros
- Line 251–255: Prints `${total_cost:.2}` — always $0.00
- Line 304: `let total_cost: f64 = episodes.iter().map(|e| f64::from(e.usage.cost_usd)).sum();`
- Line 311–315: Prints `${total_cost:.2}` — always $0.00

### Step 9: Read the serve routes that aggregate cost

File: `crates/roko-serve/src/routes/learning/mod.rs`

Run: `grep -n "cost_usd" crates/roko-serve/src/routes/learning/mod.rs | head -20`

Line 325 (approximate): `self.cost_usd += event.cost_usd;` — aggregates zeros

### Step 10: Document file paths

Run:
```bash
grep -rn 'efficiency\.jsonl\|episodes\.jsonl' crates/ --include='*.rs' | grep -v target/ | head -20
grep -rn 'cascade.router\.json\|gate.thresholds' crates/ --include='*.rs' | grep -v target/ | head -10
```

### Step 11: Write findings to `.roko/GAPS.md`

Append this section to `.roko/GAPS.md`:

```markdown
## R5_Z01: Telemetry data flow audit (audited <DATE>)

### Stream-json protocol
- `ClaudeResultEvent` has `total_cost_usd: Option<f64>` and `usage: Option<ClaudeUsage>`
- `ClaudeUsage` fields use `#[serde(default)]` on `u64` → 0 when absent in JSON
- Model name is in `ClaudeSystemEvent.model` (system init event), NOT in result event

### Bug: Usage never extracted (claude_cli_agent.rs line 660)
- `AgentResult::ok(...).with_usage(Usage { wall_ms, ..Default::default() })`
- All token counts and cost stay at 0 after every successful run
- `output_text()` extracts text but never extracts cost/usage from result event

### Efficiency events (orchestrate.rs line 17332)
- `input_tokens: u64::from(result.usage.input_tokens)` → always 0
- `output_tokens: u64::from(result.usage.output_tokens)` → always 0
- `cost_usd: f64::from(result.usage.cost_usd)` → always 0.0
- Failure path (line 17452) hardcodes zeros
- Written to `.roko/learn/efficiency.jsonl`

### Episode writes (orchestrate.rs)
- Success episode line 10497: `cost_usd: f64::from(result.usage.cost_usd)` → 0
- Failure episode line 12614: same pattern
- Episodes use `ep.model = model.to_string()` or `"unknown-model"` sentinel
- Written to `.roko/episodes.jsonl`

### "unknown-model" occurrences
<LIST FROM GREP OUTPUT>

### Two Usage structs
- `roko_core::chat_types::Usage`: u32 tokens, f32 cost
- `roko_learn::episode_logger::Usage`: u64 tokens, f64 cost (different!)

### Dashboard display
- `learn all` CLI: sums `cost_usd` which is always 0.0 → shows "$0.00"
- Serve routes: aggregate `event.cost_usd` which is always 0.0

### File paths
- `.roko/learn/efficiency.jsonl`: written by `append_efficiency_event`, read by `read_project_efficiency_events`
- `.roko/episodes.jsonl`: written by `append_episode` + `EpisodeLogger::append`, read by `read_project_episodes_lossy`
- `.roko/learn/cascade-router.json`: written by `CascadeRouter::save`, read by `print_learn_router`
```

## Acceptance Criteria

- [ ] Verified stream-json protocol structs with actual line numbers from `stream.rs`
- [ ] Verified the usage-not-extracted bug at `claude_cli_agent.rs` line 660
- [ ] Verified efficiency event zero values at `orchestrate.rs` lines 17339–17344
- [ ] All "unknown-model" occurrences listed (run grep, document all files:lines)
- [ ] Two `Usage` structs documented (u32/f32 vs u64/f64)
- [ ] CLI learn display lines documented (learn.rs lines 240, 304)
- [ ] File paths mapped (write sites → read sites)
- [ ] Findings appended to `.roko/GAPS.md`
- [ ] No source files modified

## Verification

```bash
# Confirm no source file changes
git diff --name-only crates/ | head -5
# Expected: no output

# Confirm GAPS.md updated
grep "R5_Z01" .roko/GAPS.md && echo "OK" || echo "MISSING"

# Verify grepped data (run these to get actual line numbers for documentation)
grep -n "struct ClaudeResultEvent\|struct ClaudeUsage" crates/roko-agent/src/provider/claude_cli/stream.rs
grep -n "with_usage\|Default::default" crates/roko-agent/src/claude_cli_agent.rs
grep -rn 'unknown-model' crates/ --include='*.rs' | grep -v target/
grep -rn 'efficiency\.jsonl\|episodes\.jsonl' crates/ --include='*.rs' | grep -v target/ | head -20
```

## Do NOT
- Modify any source files
- Propose solutions (that is for R5_A01–R5_A05)
- Skip any crate that touches telemetry
- Assume file formats without reading actual files
- Delete existing `.roko/GAPS.md` content — only append

---

## Read-Only Context (do not modify)

### `crates/roko-agent/src/claude_cli_agent.rs` (1232 lines — signatures only)

```rust
34:pub fn build_settings_json() -> String {
84:pub struct ClaudeCliAgent {
104:impl ClaudeCliAgent {
107:    pub fn new(
136:    pub fn with_name(mut self, name: impl Into<String>) -> Self {
143:    pub const fn with_timeout_ms(mut self, timeout_ms: u64) -> Self {
150:    pub fn with_effort(mut self, effort: impl Into<String>) -> Self {
157:    pub fn with_fallback_model(mut self, fallback_model: impl Into<String>) -> Self {
164:    pub const fn with_bare_mode(mut self, bare_mode: bool) -> Self {
171:    pub fn with_system_prompt(mut self, prompt: impl Into<String>) -> Self {
178:    pub fn with_tools(mut self, tools: impl Into<String>) -> Self {
185:    pub fn with_allowed_tools(mut self, tools: impl Into<String>) -> Self {
192:    pub const fn with_max_turns(mut self, max_turns: u32) -> Self {
199:    pub fn with_settings_json(mut self, json: impl Into<String>) -> Self {
206:    pub fn with_extra_args<I, S>(mut self, args: I) -> Self
217:    pub fn with_env_var(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
224:    pub fn with_mcp_config(mut self, path: impl Into<PathBuf>) -> Self {
231:    pub fn with_resume(mut self, session_id: impl Into<String>) -> Self {
238:    pub fn with_optional_resume(mut self, session_id: Option<String>) -> Self {
245:    pub const fn with_dangerously_skip_permissions(mut self, enabled: bool) -> Self {
617:impl Agent for ClaudeCliAgent {
868:enum UsageSource {
873:impl Default for UsageSource {
881:struct StreamUsage {
891:impl StreamUsage {
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

### `crates/roko-learn/src/runtime_feedback.rs` (4911 lines — signatures only)

```rust
63:struct EpisodeActions {
68:impl EpisodeActions {
77:fn affect_state_path(learn_root: &Path) -> PathBuf {
83:struct GateCounts {
89:impl GateCounts {
115:impl EpisodeView for EpisodeActions {
126:pub struct LearningPaths {
128:    pub root: PathBuf,
130:    pub episodes_jsonl: PathBuf,
132:    pub costs_jsonl: PathBuf,
134:    pub skills_json: PathBuf,
136:    pub playbooks_dir: PathBuf,
138:    pub playbook_rules_toml: PathBuf,
140:    pub task_metrics_jsonl: PathBuf,
142:    pub efficiency_jsonl: PathBuf,
144:    pub efficiency_summaries_jsonl: PathBuf,
146:    pub gate_outcomes_jsonl: PathBuf,
148:    pub retry_outcomes_jsonl: PathBuf,
150:    pub knowledge_seeds_jsonl: PathBuf,
152:    pub latency_stats_json: PathBuf,
154:    pub cfactor_jsonl: PathBuf,
156:    pub cascade_router_json: PathBuf,
158:    pub experiments_json: PathBuf,
160:    pub experiment_winners_json: PathBuf,
162:    pub gate_thresholds_json: PathBuf,
164:    pub local_rewards_json: PathBuf,
166:    pub section_effects_json: PathBuf,
168:    pub post_gate_reflections_json: PathBuf,
170:    pub provider_model_outcomes_jsonl: PathBuf,
173:impl LearningPaths {
176:    pub fn under(root: impl Into<PathBuf>) -> Self {
207:pub struct RegressionConfig {
209:    pub thresholds: RegressionThresholds,
211:    pub current_window: usize,
214:impl Default for RegressionConfig {
226:pub struct UpdateFrequency {
228:    pub router_every_n_episodes: u32,
230:    pub gate_thresholds_every_n: u32,
232:    pub experiments_every_n: u32,
234:    pub skill_mining_every_n: u32,
```

### `crates/roko-learn/src/cascade_router.rs` (2195 lines — signatures only)

```rust
39:pub use crate::cascade::helpers::slug_family;
40:pub use crate::cascade::types::{
82:pub struct CascadeRouter {
99:impl roko_core::Cell for CascadeRouter {
111:impl Default for RoutingContext {
134:impl roko_agent::model_call_service::ForceBackendOverrideRecorder for CascadeRouter {
145:impl CascadeRouter {
151:    pub fn new(model_slugs: Vec<String>) -> Self {
172:    pub fn with_role_table(mut self, table: HashMap<AgentRole, String>) -> Self {
178:    pub fn set_static_role_model(&mut self, role: AgentRole, model_slug: impl Into<String>) {
183:    pub fn update_static_table(&self, role: AgentRole, model_slug: impl Into<String>) -> bool {
198:    pub fn with_linucb(mut self, linucb: LinUCBRouter) -> Self {
205:    pub fn with_free_tier_shadow_runner(mut self, runner: Arc<dyn ShadowModelRunner>) -> Self {
212:    pub fn current_stage(&self) -> CascadeStage {
218:    pub fn model_slugs(&self) -> &[String] {
224:    pub fn total_observations(&self) -> u64 {
229:    pub fn check_stage_transition(&self) -> Option<StageTransition> {
263:    pub fn stage_transitions(&self) -> Vec<StageTransition> {
269:    pub fn select(&self, context_vec: Vec<f64>) -> CascadeSelection {
282:    pub fn select_for_frequency(
302:    pub fn select_for_frequency_among(
333:    pub fn select_tier_with_active_inference(
344:    pub fn strongest_model(&self) -> ModelSpec {
365:    pub fn cheapest_model(&self) -> ModelSpec {
386:    pub fn strongest_model_among(&self, candidates: &[String]) -> ModelSpec {
406:    pub fn cheapest_model_among(&self, candidates: &[String]) -> ModelSpec {
584:    pub fn model_index_for_slug(&self, slug: &str) -> Option<usize> {
589:    pub fn route(&self, ctx: &RoutingContext) -> CascadeModel {
594:    pub fn route_logged(
608:    pub fn route_with_experiments(
628:    pub fn route_with_health(
659:    pub fn filter_unhealthy(
709:    pub fn apply_bias(&self, candidates: &mut [(String, f64)], bias: &RoutingBias) {
730:    pub fn apply_cost_pressure(&self, candidates: &mut [(String, f64)], spike: bool) {
741:    pub fn route_with_bias(&self, ctx: &RoutingContext, bias: &RoutingBias) -> CascadeModel {
757:    pub fn load_static_overrides(&self, path: &Path) -> std::io::Result<usize> {
779:    pub fn latency_penalty(actual_ms: f64, expected_ms: f64) -> f64 {
788:    pub fn reward_with_latency(
801:    pub fn reward_with_tracker_latency(
815:    pub fn route_with_cfactor(
```

### `crates/roko-cli/src/commands/learn.rs`

```rust
//! learn command handlers.
#![allow(unused_imports)]

use crate::*;

pub(crate) async fn dispatch_learn(cli: &Cli, cmd: LearnCmd) -> Result<i32> {
    match cmd {
        LearnCmd::All { workdir } => {
            let wd = workdir.unwrap_or_else(|| resolve_workdir(cli));
            cmd_learn(&wd, "all").await
        }
        LearnCmd::Route { workdir } => {
            let wd = workdir.unwrap_or_else(|| resolve_workdir(cli));
            cmd_learn(&wd, "router").await
        }
        LearnCmd::Experiments { workdir } => {
            let wd = workdir.unwrap_or_else(|| resolve_workdir(cli));
            cmd_learn(&wd, "experiments").await
        }
        LearnCmd::Efficiency { workdir } => {
            let wd = workdir.unwrap_or_else(|| resolve_workdir(cli));
            cmd_learn(&wd, "efficiency").await
        }
        LearnCmd::Episodes { workdir } => {
            let wd = workdir.unwrap_or_else(|| resolve_workdir(cli));
            cmd_learn(&wd, "episodes").await
        }
        LearnCmd::Tune {
            subsystem,
            dry_run,
            workdir,
        } => {
            let wd = workdir.unwrap_or_else(|| resolve_workdir(cli));
            cmd_tune(&wd, &subsystem, dry_run).await
        }
    }
}

/// `roko tune [subsystem]` — display and optionally adjust adaptive thresholds.
pub(crate) async fn cmd_tune(
    workdir: &std::path::Path,
    subsystem: &str,
    dry_run: bool,
) -> Result<i32> {
    match subsystem {
        "gates" => {
            let path = learn_gate_thresholds_path(workdir);
            if path.exists() {
                let content = std::fs::read_to_string(&path)?;
                let thresholds: serde_json::Value = serde_json::from_str(&content)?;
                println!("Verify adaptive thresholds ({}):", path.display());
                println!("{}", serde_json::to_string_pretty(&thresholds)?);
            } else {
                print_no_data(&path);
            }
        }
        "routing" => {
            let path = learn_router_path(workdir);
            if path.exists() {
                let content = std::fs::read_to_string(&path)?;
                let router: serde_json::Value = serde_json::from_str(&content)?;
                println!("Cascade router state ({}):", path.display());
                println!("{}", serde_json::to_string_pretty(&router)?);
            } else {
                print_no_data(&path);
            }
        }
        "budget" => {
            let path = learn_efficiency_path(workdir);
            if path.exists() {
                let content = std::fs::read_to_string(&path)?;
                let count = content.lines().filter(|l| !l.trim().is_empty()).count();
                println!("Efficiency log: {} entries at {}", count, path.display());
            } else {
                print_no_data(&path);
            }
        }
        other => {
            eprintln!("Unknown subsystem '{other}'. Available: gates, routing, budget");
            return Ok(1);
        }
    }
    if dry_run {
        println!("(dry-run: no changes applied)");
    }
    Ok(EXIT_SUCCESS)
}

/// `roko learn [what]` — display learning subsystem state.
pub(crate) async fn cmd_learn(workdir: &std::path::Path, what: &str) -> Result<i32> {
    let show_all = what == "all";

    if show_all || what == "router" {
        print_learn_router(workdir);
    }

    if show_all || what == "experiments" {
        print_learn_experiments(workdir);
    }

    if show_all || what == "efficiency" {
        print_learn_efficiency(workdir).await;
    }

    if show_all || what == "episodes" {
        print_learn_episodes(workdir).await;
    }

    if show_all {
        print_learn_knowledge(workdir).await;
    }

    if !show_all && !["router", "experiments", "efficiency", "episodes"].contains(&what) {
        eprintln!(
            "Unknown learning area '{what}'. Available: router, experiments, efficiency, episodes, all"
        );
        return Ok(1);
    }

    Ok(EXIT_SUCCESS)
}

pub(crate) fn print_learn_router(workdir: &std::path::Path) {
    let path = learn_router_path(workdir);
    if !path.exists() {
        print_no_data(&path);
        return;
    }
    let Ok(content) = std::fs::read_to_string(&path) else {
        print_no_data(&path);
        return;
    };
    let snapshot = serde_json::from_str::<LearnCascadeRouterSnapshot>(&content).unwrap_or_default();

    let mut first_seen: Option<chrono::DateTime<chrono::Utc>> = None;
    let mut last_seen: Option<chrono::DateTime<chrono::Utc>> = None;
    for transition in &snapshot.stage_transitions {
        first_seen = Some(match first_seen {
            Some(current) => current.min(transition.timestamp.clone()),
            None => transition.timestamp.clone(),
        });
        last_seen = Some(match last_seen {
            Some(current) => current.max(transition.timestamp.clone()),
            None => transition.timestamp.clone(),
        });
    }

    let latest = snapshot
        .stage_transitions
        .last()
        .map(|transition| {
            format!(
                "{} {} -> {} after {} observations",
                transition.timestamp.to_rfc3339(),
                transition.from,
                transition.to,
                transition.observations
            )
        })
        .unwrap_or_else(|| {
            format!(
                "snapshot stage={} total_observations={}",
                cascade_stage_for_observations(snapshot.total_observations),
                snapshot.total_observations
            )
        });

    println!(
        "Cascade router: {} observations, {} models at {}",
        snapshot.total_observations,
        snapshot.model_slugs.len(),
        path.display()
    );
    println!("  Range: {}", format_range(first_seen, last_seen));
    println!("  Latest: {}", latest);
}

pub(crate) fn print_learn_experiments(workdir: &std::path::Path) {
    // Prompt experiments
    let prompt_path = learn_root(workdir).join("experiments.json");
    let prompt_store = ExperimentStore::load_or_new(&prompt_path);
    let running = prompt_store.running_count();
    let concluded = prompt_store.concluded_count();
    if running > 0 || concluded > 0 {
        println!(
            "Prompt experiments: {} running, {} concluded",
            running, concluded
        );
    } else {
        println!("Prompt experiments: none");
    }

    // Model experiments
    let model_path = learn_root(workdir).join("model-experiments.json");
    let model_store = roko_learn::model_experiment::ModelExperimentStore::load_or_new(&model_path);
    let model_running = model_store.running_count();
    let model_concluded = model_store.concluded_experiments().len();
    if model_running > 0 || model_concluded > 0 {
        println!(
            "Model experiments: {} running, {} concluded",
            model_running, model_concluded
        );
        for exp in model_store.iter() {
            println!(
                "  {} [{:?}] role={} variants={} winner={}",
                exp.experiment_id,
                exp.status,
                exp.role.as_deref().unwrap_or("any"),
                exp.variants.len(),
                exp.winner_id.as_deref().unwrap_or("-"),
            );
        }
    } else {
        println!("Model experiments: none");
    }
}

#[allow(clippy::cast_precision_loss)]
pub(crate) async fn print_learn_efficiency(workdir: &std::path::Path) {
    let path = learn_efficiency_path(workdir);
    if !path.exists() {
        print_no_data(&path);
        return;
    }

    let Ok(text) = tokio::fs::read_to_string(&path).await else {
        print_no_data(&path);
        return;
    };

    let mut count = 0usize;
    let mut first_seen: Option<chrono::DateTime<chrono::Utc>> = None;
    let mut last_seen: Option<chrono::DateTime<chrono::Utc>> = None;
    let mut latest: Option<String> = None;

    for line in text.lines() {
        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }
        let Ok(event) = serde_json::from_str::<roko_learn::efficiency::AgentEfficiencyEvent>(trimmed)
        else {
            continue;
        };

        count += 1;
        let parsed_timestamp = parse_rfc3339_utc(&event.timestamp);
        if let Some(timestamp) = parsed_timestamp {
            first_seen = Some(match first_seen {
                Some(current) => current.min(timestamp),
                None => timestamp,
            });
            last_seen = Some(match last_seen {
                Some(current) => current.max(timestamp),
                None => timestamp,
            });
        }

        let timestamp = parsed_timestamp
            .map(|ts| ts.to_rfc3339())
            .unwrap_or_else(|| event.timestamp.clone());
        let model = efficiency_model_label(&event);
        let task_id = non_empty_or_unknown(&event.task_id);
        let plan_id = non_empty_or_unknown(&event.plan_id);
        let status = if event.gate_passed { "pass" } else { "fail" };
        latest = Some(format!(
            "{timestamp} model={model} task={task_id} plan={plan_id} {status} cost=${:.4}",
            event.cost_usd
        ));
    }

    println!("Efficiency: {} events at {}", count, path.display());
    println!("  Range: {}", format_range(first_seen, last_seen));
    println!(
        "  Latest: {}",
        latest.unwrap_or_else(|| "none".to_string())
    );
}

pub(crate) async fn print_learn_episodes(workdir: &std::path::Path) {
    let path = learn_episodes_path(workdir);
    if !path.exists() {
        print_no_data(&path);
        return;
    }

    let Ok(text) = tokio::fs::read_to_string(&path).await else {
        print_no_data(&path);
        return;
    };

    let mut count = 0usize;
    let mut first_seen: Option<chrono::DateTime<chrono::Utc>> = None;
    let mut last_seen: Option<chrono::DateTime<chrono::Utc>> = None;
    let mut latest: Option<String> = None;

    for line in text.lines() {
        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }
        let Ok(episode) = serde_json::from_str::<roko_learn::episode_logger::Episode>(trimmed)
        else {
            continue;
        };

        count += 1;
        first_seen = Some(match first_seen {
            Some(current) => current.min(episode.timestamp.clone()),
            None => episode.timestamp.clone(),
        });
        last_seen = Some(match last_seen {
            Some(current) => current.max(episode.timestamp.clone()),
            None => episode.timestamp.clone(),
        });

        let status = if episode.success { "pass" } else { "fail" };
        let model = non_empty_or_unknown(&episode.model);
        let task_id = non_empty_or_unknown(&episode.task_id);
        latest = Some(format!(
            "{} model={model} task={task_id} {status} cost=${:.4}",
            episode.timestamp.to_rfc3339(),
            episode.usage.cost_usd
        ));
    }

    println!("Episodes: {} entries at {}", count, path.display());
    println!("  Range: {}", format_range(first_seen, last_seen));
    println!(
        "  Latest: {}",
        latest.unwrap_or_else(|| "none".to_string())
    );
}

pub(crate) async fn print_learn_knowledge(workdir: &std::path::Path) {
    let path = learn_knowledge_path(workdir);
    if !path.exists() {
        print_no_data(&path);
        return;
    }
    let Ok(content) = tokio::fs::read_to_string(&path).await else {
        print_no_data(&path);
        return;
    };
    let count = content
        .lines()
        .filter(|line| !line.trim().is_empty())
        .filter(|line| serde_json::from_str::<serde_json::Value>(line).is_ok())
        .count();
    println!("Knowledge: {} durable entries at {}", count, path.display());
}

fn learn_root(workdir: &std::path::Path) -> std::path::PathBuf {
    workdir.join(".roko").join("learn")
}

fn learn_gate_thresholds_path(workdir: &std::path::Path) -> std::path::PathBuf {
    learn_root(workdir).join("gate-thresholds.json")
}

fn learn_router_path(workdir: &std::path::Path) -> std::path::PathBuf {
    learn_root(workdir).join("cascade-router.json")
}

fn learn_efficiency_path(workdir: &std::path::Path) -> std::path::PathBuf {
    learn_root(workdir).join("efficiency.jsonl")
}

fn learn_episodes_path(workdir: &std::path::Path) -> std::path::PathBuf {
    learn_root(workdir).join("episodes.jsonl")
}

fn learn_knowledge_path(workdir: &std::path::Path) -> std::path::PathBuf {
    workdir.join(".roko").join("neuro").join("knowledge.jsonl")
}

fn print_no_data(path: &std::path::Path) {
    println!("No data at {}", path.display());
}

fn parse_rfc3339_utc(timestamp: &str) -> Option<chrono::DateTime<chrono::Utc>> {
    chrono::DateTime::parse_from_rfc3339(timestamp)
        .ok()
        .map(|parsed| parsed.with_timezone(&chrono::Utc))
}

fn format_range(
    first_seen: Option<chrono::DateTime<chrono::Utc>>,
    last_seen: Option<chrono::DateTime<chrono::Utc>>,
) -> String {
    match (first_seen, last_seen) {
        (Some(first_seen), Some(last_seen)) => {
            format!("{} .. {}", first_seen.to_rfc3339(), last_seen.to_rfc3339())
        }
        _ => "n/a".to_string(),
    }
}

fn non_empty_or_unknown(value: &str) -> &str {
    let trimmed = value.trim();
    if trimmed.is_empty() { "unknown" } else { trimmed }
}

fn efficiency_model_label(event: &roko_learn::efficiency::AgentEfficiencyEvent) -> &str {
    let model_used = event.model_used.trim();
    if model_used.is_empty() {
        non_empty_or_unknown(&event.model)
    } else {
        model_used
    }
}

fn cascade_stage_for_observations(observations: u64) -> &'static str {
    if observations >= 200 {
        "ucb"
    } else if observations >= 50 {
        "confidence"
    } else {
        "static"
    }
}

#[derive(Default, serde::Deserialize)]
struct LearnCascadeRouterSnapshot {
    #[serde(default)]
    model_slugs: Vec<String>,
    #[serde(default)]
    total_observations: u64,
    #[serde(default)]
    stage_transitions: Vec<roko_learn::cascade::StageTransition>,
}
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
