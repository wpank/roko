# AUDIT: Batch R2_E01 — Map learn write/read paths

Run: run-20260429-030528 | Phase: AUDIT | Model: codex-5.5

## Your Role

You are an **auditor**. A fast implementation agent just completed batch `R2_E01`.
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

Map learn write/read paths

## Runner Context

You are working in runner `mega-parity`, batch R2_E01.
This batch is part of Runner 2: execution-contract — Make CLI execution contracts truthful enough that demo scenarios and agent sessions can rely on them.

## Problem

The learning subsystem writes data during execution (efficiency events, episodes, cost
data, cascade router state) and reads it back via `roko learn` commands. The write
paths and read paths must match — learn commands may read from different files than
`orchestrate.rs` writes to, showing stale or empty data.

This is a context-only batch. Produce a reference document that R2_E02 depends on.
The context document has ALREADY BEEN RESEARCHED below. All path mappings are confirmed
from the actual codebase. Do NOT re-research — just write the output document.

## Pre-Researched Path Mappings (use these directly)

All paths below are confirmed from reading the actual source files.

| Data type         | Write path                                        | Code location (write)                           | Read path                                         | Code location (read)                     | Match   |
|-------------------|---------------------------------------------------|-------------------------------------------------|---------------------------------------------------|------------------------------------------|---------|
| Efficiency events | `<workdir>/.roko/learn/efficiency.jsonl`          | `orchestrate.rs:5376,7227` via `LearningPaths`  | `<workdir>/.roko/learn/efficiency.jsonl`          | `learn.rs:228` → `runtime_feedback.rs:2897` | YES |
| Episodes          | `<workdir>/.roko/episodes.jsonl`                  | `orchestrate.rs:11141,17772` `EpisodeLogger::new` | Checks 3 paths; `.roko/episodes.jsonl` is #2   | `learn.rs:290` → `runtime_feedback.rs:2870,2855` | YES |
| Cost data         | `<workdir>/.roko/learn/costs.jsonl`               | `LearningPaths` `costs_jsonl` field             | NOT read by `learn` subcommands; `status` reads it | `commands/util.rs:439` `CostsLog::at`   | PARTIAL |
| Cascade router    | `<workdir>/.roko/learn/cascade-router.json`       | `orchestrate.rs:5317-5321`                      | `<workdir>/.roko/learn/cascade-router.json`       | `learn.rs:125` `print_learn_router`      | YES |

**Missing-file messages (current problems for R2_E02):**
- `print_learn_efficiency` (learn.rs line 230): prints `"Efficiency log: empty"` — does NOT name the path
- `print_learn_episodes` (learn.rs line 292): prints `"Episodes: none"` — does NOT name the path
- `print_learn_router` (learn.rs line 126): prints `"Cascade router: not initialized"` — does NOT name the path

## Step-by-Step Instructions

### Step 1: Locate the efficiency write path

Run this grep to confirm the path:

```bash
grep -n "efficiency.jsonl\|append_efficiency_event" \
  /Users/will/dev/nunchi/roko/roko/crates/roko-cli/src/orchestrate.rs | head -20
```

Expected: line 17400 calls `self.learning.append_efficiency_event(&event)`.
The `LearningRuntime` is initialized at line 3938–3948:
```rust
let learn_root = workdir.join(".roko").join("learn");
```
`LearningRuntime::open_under(learn_root)` builds `LearningPaths::under(root)` at
`crates/roko-learn/src/runtime_feedback.rs` line 152:
```rust
efficiency_jsonl: root.join("efficiency.jsonl"),
```
So `append_efficiency_event` writes to:
**`<workdir>/.roko/learn/efficiency.jsonl`**

### Step 2: Locate the efficiency read path

Run this grep to confirm:

```bash
grep -n "read_project_efficiency_events\|efficiency.jsonl" \
  /Users/will/dev/nunchi/roko/roko/crates/roko-cli/src/commands/learn.rs
```

Expected: line 228 calls `roko_learn::runtime_feedback::read_project_efficiency_events(workdir)`.
That function is at `crates/roko-learn/src/runtime_feedback.rs` line 2894–2897:
```rust
pub async fn read_project_efficiency_events(
    workdir: impl AsRef<Path>,
) -> Result<Vec<AgentEfficiencyEvent>, LearningRuntimeError> {
    read_efficiency_events(&workdir.as_ref().join(".roko/learn/efficiency.jsonl")).await
}
```
Read path: **`<workdir>/.roko/learn/efficiency.jsonl`**
**Match: YES.**

### Step 3: Locate the episodes write path

Run this grep:

```bash
grep -n "episodes.jsonl\|append_episode\|EpisodeLogger::new" \
  /Users/will/dev/nunchi/roko/roko/crates/roko-cli/src/orchestrate.rs | head -20
```

Expected results:
- Line 7322: `let episodes_path = self.workdir.join(".roko").join("episodes.jsonl");`
- Line 11141: `EpisodeLogger::new(self.workdir.join(".roko").join("episodes.jsonl"))`
- Line 17771–17772: `EpisodeLogger::new(self.workdir.join(".roko").join("episodes.jsonl"))`

Write path: **`<workdir>/.roko/episodes.jsonl`**

### Step 4: Locate the episodes read path

Run this grep:

```bash
grep -n "read_project_episodes_lossy\|project_episode_paths" \
  /Users/will/dev/nunchi/roko/roko/crates/roko-learn/src/runtime_feedback.rs | head -20
```

Expected: `project_episode_paths` (line 2855) returns three paths in priority order:
```rust
vec![
    roko.join("memory").join("episodes.jsonl"),   // not written by orchestrate
    roko.join("episodes.jsonl"),                   // ← orchestrate writes here
    roko.join("learn").join("episodes.jsonl"),     // fallback legacy
]
```
`learn.rs` line 290 calls `read_project_episodes_lossy(workdir)` which reads all three paths and deduplicates.

Read paths (checked in order):
1. `<workdir>/.roko/memory/episodes.jsonl`
2. `<workdir>/.roko/episodes.jsonl`  ← matches write path
3. `<workdir>/.roko/learn/episodes.jsonl`

**Match: YES** (orchestrate writes to `.roko/episodes.jsonl`; reader checks that path as second priority).

### Step 5: Locate the cascade router write path

Run this grep:

```bash
grep -n "save_cascade_router\|cascade.router.json\|cascade_router_json" \
  /Users/will/dev/nunchi/roko/roko/crates/roko-cli/src/orchestrate.rs | head -10
```

Expected: line 5314–5321:
```rust
if let Err(e) = self.learning.save_cascade_router() { ... }
// ...
.join("cascade-router.json");
```

`LearningPaths::under` (runtime_feedback.rs line 159):
```rust
cascade_router_json: root.join("cascade-router.json"),
```
Where `root` = `<workdir>/.roko/learn`.
Write path: **`<workdir>/.roko/learn/cascade-router.json`**

### Step 6: Locate the cascade router read path

Run this grep:

```bash
grep -n "cascade.router\|cascade_router" \
  /Users/will/dev/nunchi/roko/roko/crates/roko-cli/src/commands/learn.rs | head -10
```

Expected: line 125 (`print_learn_router`):
```rust
let path = workdir.join(".roko/learn/cascade-router.json");
```
Read path: **`<workdir>/.roko/learn/cascade-router.json`**
**Match: YES.**

### Step 7: Locate the cost data write path

Run this grep:

```bash
grep -n "costs_jsonl\|costs\.jsonl\|CostsLog" \
  /Users/will/dev/nunchi/roko/roko/crates/roko-learn/src/runtime_feedback.rs | head -10
```

Expected: `LearningPaths::under` line 147:
```rust
costs_jsonl: root.join("costs.jsonl"),
```
Write path: **`<workdir>/.roko/learn/costs.jsonl`**

### Step 8: Locate the cost data read path in learn.rs

Run this grep:

```bash
grep -n "cost\|Cost" \
  /Users/will/dev/nunchi/roko/roko/crates/roko-cli/src/commands/learn.rs | head -20
```

`learn all` reads costs indirectly via `print_learn_efficiency` (line 228) which reads
`efficiency.jsonl` (each efficiency event has a `cost_usd` field).
There is NO direct read of `costs.jsonl` in `learn.rs`.

**Match: PARTIAL** — cost is aggregated from efficiency events, not read from `costs.jsonl`.

### Step 9: Write the context document

Create the directory and file:

```bash
mkdir -p /Users/will/dev/nunchi/roko/roko/tmp/runners/mega-parity/context
```

Write to `tmp/runners/mega-parity/context/R2_E01_learn_paths.md` (the file you may modify):

```markdown
# R2_E01: Learning Path Map

## 1. Efficiency events

- **Write path**: `<workdir>/.roko/learn/efficiency.jsonl`
  - Code: `crates/roko-cli/src/orchestrate.rs:17400` calls
    `self.learning.append_efficiency_event(&event)`
  - `LearningPaths::under` at `crates/roko-learn/src/runtime_feedback.rs:152`
    maps `efficiency_jsonl = root.join("efficiency.jsonl")` where
    `root = workdir.join(".roko").join("learn")`.
- **Read path**: `<workdir>/.roko/learn/efficiency.jsonl`
  - Code: `crates/roko-cli/src/commands/learn.rs:228` calls
    `read_project_efficiency_events(workdir)` at
    `crates/roko-learn/src/runtime_feedback.rs:2897`
    which reads `workdir.join(".roko/learn/efficiency.jsonl")`.
- **Match: YES**

## 2. Episodes

- **Write path**: `<workdir>/.roko/episodes.jsonl`
  - Code: `crates/roko-cli/src/orchestrate.rs:11141` and `17771–17772`:
    `EpisodeLogger::new(self.workdir.join(".roko").join("episodes.jsonl"))`.
- **Read path**: reads three paths in order:
  1. `<workdir>/.roko/memory/episodes.jsonl`
  2. `<workdir>/.roko/episodes.jsonl`   ← matches write path
  3. `<workdir>/.roko/learn/episodes.jsonl`
  - Code: `crates/roko-cli/src/commands/learn.rs:290` calls
    `read_project_episodes_lossy(workdir)` at
    `crates/roko-learn/src/runtime_feedback.rs:2870`
    which calls `project_episode_paths` (line 2855).
- **Match: YES** (canonical write path is in the read set)

## 3. Cost data

- **Write path**: `<workdir>/.roko/learn/costs.jsonl`
  - Code: `LearningPaths::under` at `crates/roko-learn/src/runtime_feedback.rs:147`
    maps `costs_jsonl = root.join("costs.jsonl")`.
- **Read path for `learn all`**: costs are NOT read from `costs.jsonl`.
  `print_learn_efficiency` (learn.rs line 228) reads efficiency events from
  `efficiency.jsonl` and sums `cost_usd` per event.
- **Match: PARTIAL** — cost totals come from efficiency events; `costs.jsonl` is never
  read by any `learn` subcommand.

## 4. Cascade router

- **Write path**: `<workdir>/.roko/learn/cascade-router.json`
  - Code: `crates/roko-cli/src/orchestrate.rs:5314` calls
    `self.learning.save_cascade_router()` which calls
    `self.cascade_router.save(&self.paths.cascade_router_json)` at
    `crates/roko-learn/src/runtime_feedback.rs:1819`.
  - `LearningPaths::under` line 159: `cascade_router_json = root.join("cascade-router.json")`.
- **Read path**: `<workdir>/.roko/learn/cascade-router.json`
  - Code: `crates/roko-cli/src/commands/learn.rs:125` (`print_learn_router`):
    `workdir.join(".roko/learn/cascade-router.json")`.
- **Match: YES**
```

## Write Scope

- `tmp/runners/mega-parity/context/R2_E01_learn_paths.md` (create this file)

## Read-Only Context (do not modify these)

- `crates/roko-cli/src/orchestrate.rs`
- `crates/roko-cli/src/commands/learn.rs`
- `crates/roko-learn/src/runtime_feedback.rs`

## Acceptance Criteria

- [ ] Document exists at `tmp/runners/mega-parity/context/R2_E01_learn_paths.md`
- [ ] Efficiency events: write path documented with exact file:line (`orchestrate.rs:17400`)
- [ ] Efficiency events: read path documented with exact file:line (`runtime_feedback.rs:2897`)
- [ ] Efficiency events: match status = YES
- [ ] Episodes: write path documented with exact file:line (`orchestrate.rs:11141`)
- [ ] Episodes: read path documented with function name + 3 checked paths
- [ ] Episodes: match status = YES
- [ ] Cost data: write path documented (`costs.jsonl`)
- [ ] Cost data: match status = PARTIAL, with explanation
- [ ] Cascade router: write path documented (`cascade-router.json`)
- [ ] Cascade router: read path documented with exact file:line (`learn.rs:125`)
- [ ] Cascade router: match status = YES
- [ ] No source code was modified

## Verification

```bash
# Verify the document was created
ls /Users/will/dev/nunchi/roko/roko/tmp/runners/mega-parity/context/R2_E01_learn_paths.md

# Verify write paths by grepping orchestrate.rs
grep -n "efficiency.jsonl\|episodes.jsonl\|cascade-router\|save_cascade_router" \
  /Users/will/dev/nunchi/roko/roko/crates/roko-cli/src/orchestrate.rs | head -20

# Verify read paths by grepping learn.rs
grep -n "efficiency\|episodes\|cascade" \
  /Users/will/dev/nunchi/roko/roko/crates/roko-cli/src/commands/learn.rs | head -20
```

## Do NOT

- Change any source code
- Guess at paths — all paths confirmed from reading actual code above
- Document aspirational behavior

## Evidence

COMPREHENSIVE-ISSUES 4.1

---

## Read-Only Context (do not modify)

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
