# AUDIT: Batch R5_A05 — Display unknown instead of $0.00 for null cost

Run: run-20260429-030528 | Phase: AUDIT | Model: codex-5.5

## Your Role

You are an **auditor**. A fast implementation agent just completed batch `R5_A05`.
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
Display "unknown" instead of "$0.00" for null cost

## Runner Context
You are working in runner `mega-parity`, batch R5_A05.
This batch is part of Runner 5: telemetry-learning — Make cost, usage, episodes, learning, and cascade router feedback truthful enough that dashboards show real data and self-improvement actually works.

## Problem
The CLI display code (`learn.rs`) and the HTTP API both display cost as a formatted float. Before R5_A01/R5_A03, all costs were zero because usage was never populated. After those batches, most costs will be real values. However, for any runs where cost is genuinely not available (e.g., timed-out agents, old JSONL entries), the display will show `$0.00` instead of `unknown`.

The issue: `AgentEfficiencyEvent.cost_usd: f64` and `Episode.usage.cost_usd: f64` both use `0.0` for "not available." There is currently no way to distinguish "genuinely free" from "not reported." Until R5_Z02's `UsageObservation` is threaded everywhere (a future batch), we apply a pragmatic heuristic: cost of exactly `0.0` when both input_tokens and output_tokens are also `0` is treated as "unknown."

## Architecture Contract
- `cost_usd == 0.0 AND input_tokens == 0 AND output_tokens == 0` → display as "unknown" / "-"
- `cost_usd == 0.0 AND (input_tokens > 0 OR output_tokens > 0)` → display as "$0.00" (genuinely reported free)
- `cost_usd > 0.0` → display as "$X.XX"
- JSON API: no change to the stored values — display only

## Write Scope (files you may modify)
- `crates/roko-cli/src/commands/learn.rs`
- `crates/roko-cli/src/status.rs` (if it displays cost — see below)

## Read-Only Context (do not modify these)
- `crates/roko-agent/src/usage.rs` — `UsageObservation` type (from R5_A02)
- `crates/roko-cli/src/orchestrate.rs` — writes the data
- `.roko/learn/efficiency.jsonl` — stored format
- `demo/demo-app/` — frontend (do not touch)

## Exact source state before this change

### `print_learn_efficiency` in `crates/roko-cli/src/commands/learn.rs` (lines 226-287)

**Display of total cost** (line 250-255):
```rust
println!(
    "Efficiency: {} events, ${:.2} total, {:.0}% pass rate",
    events.len(),
    total_cost,    // ← shows "$0.00" when all costs are zero
    pass_rate
);
```

**Per-model cost display** (line 279-286):
```rust
for (model, (count, cost, passed)) in &model_summary {
    // ...
    println!("  {model}: {count} runs, ${cost:.2}, {model_pass:.0}% pass",);
    //                           ↑ shows "$0.00" when cost is zero
}
```

### `print_learn_episodes` in `crates/roko-cli/src/commands/learn.rs` (lines 289-355)

**Total cost display** (line 311-316):
```rust
println!(
    "Episodes: {} recorded, {:.0}% success, ${:.2} total cost",
    episodes.len(),
    pass_rate,
    total_cost,    // ← same issue
);
```

**Recent episodes display** (lines 343-352):
```rust
for ep in episodes.iter().rev().take(last_n) {
    let status = if ep.success { "pass" } else { "fail" };
    println!(
        "    {} {} {} [{}] ${:.4}",
        ep.timestamp.format("%Y-%m-%d %H:%M"),
        ep.model,
        ep.task_id,
        status,
        ep.usage.cost_usd,    // ← shows "$0.0000" when zero
    );
}
```

### `status.rs` cost display

The `SessionStatus.display_text()` in `crates/roko-cli/src/status.rs` (lines 94-99):
```rust
if let Some(cost) = self.total_cost_usd {
    lines.push(format!("total cost: ${cost:.4}"));
}
if let Some(cost) = self.today_cost_usd {
    lines.push(format!("today cost: ${cost:.4}"));
}
```

`total_cost_usd` and `today_cost_usd` are `Option<f64>` — when `None` they are not printed at all (correct behavior). No change needed in `status.rs`.

### HTTP API serve routes

The serve routes aggregate `event.cost_usd` (non-Option `f64`). The JSON API response includes `cost_usd: f64` (always a number). The JSON spec does not require `null` here — the API consumers already receive `0.0` today. Out of scope for this batch.

## Changes Required

### Step 1: Add a display helper in `learn.rs`

Add the following helper function at the top of `crates/roko-cli/src/commands/learn.rs`, before `dispatch_learn`:

```rust
/// Format a cost value for human display.
/// Uses the heuristic: if cost is exactly 0.0 AND both token counts are 0,
/// treat as "unknown" (the provider did not report usage).
fn display_cost(cost_usd: f64, input_tokens: u64, output_tokens: u64) -> String {
    if cost_usd == 0.0 && input_tokens == 0 && output_tokens == 0 {
        "unknown".to_string()
    } else {
        format!("${cost_usd:.2}")
    }
}

/// Format a cost value for recent-episode display (4 decimal places).
fn display_cost_precise(cost_usd: f64, input_tokens: u64, output_tokens: u64) -> String {
    if cost_usd == 0.0 && input_tokens == 0 && output_tokens == 0 {
        "unknown".to_string()
    } else {
        format!("${cost_usd:.4}")
    }
}
```

### Step 2: Update `print_learn_efficiency` to use the helper

**Change 1**: The total cost display (lines 250-255).

**Before**:
```rust
let total_cost: f64 = events.iter().map(|e| e.cost_usd).sum();
let total_input: u64 = events.iter().map(|e| e.input_tokens).sum();
let total_output: u64 = events.iter().map(|e| e.output_tokens).sum();
// ...
println!(
    "Efficiency: {} events, ${:.2} total, {:.0}% pass rate",
    events.len(),
    total_cost,
    pass_rate
);
```

**After**:
```rust
let total_cost: f64 = events.iter().map(|e| e.cost_usd).sum();
let total_input: u64 = events.iter().map(|e| e.input_tokens).sum();
let total_output: u64 = events.iter().map(|e| e.output_tokens).sum();
// ...
println!(
    "Efficiency: {} events, {} total, {:.0}% pass rate",
    events.len(),
    display_cost(total_cost, total_input, total_output),
    pass_rate
);
```

**Change 2**: The per-model cost display (lines 279-286).

To use the helper, we need per-model token totals. Update the aggregation hashmap to track tokens:

**Before**:
```rust
let mut by_model: std::collections::HashMap<&str, (usize, f64, usize)> =
    std::collections::HashMap::new();
for ev in &events {
    let entry = by_model.entry(ev.model.as_str()).or_default();
    entry.0 += 1;
    entry.1 += ev.cost_usd;
    if ev.gate_passed {
        entry.2 += 1;
    }
}
// ...
for (model, (count, cost, passed)) in &model_summary {
    // ...
    println!("  {model}: {count} runs, ${cost:.2}, {model_pass:.0}% pass",);
}
```

**After**:
```rust
// (usize count, f64 cost, usize passed, u64 input_tokens, u64 output_tokens)
let mut by_model: std::collections::HashMap<&str, (usize, f64, usize, u64, u64)> =
    std::collections::HashMap::new();
for ev in &events {
    let entry = by_model.entry(ev.model.as_str()).or_default();
    entry.0 += 1;
    entry.1 += ev.cost_usd;
    if ev.gate_passed {
        entry.2 += 1;
    }
    entry.3 += ev.input_tokens;
    entry.4 += ev.output_tokens;
}
let mut model_summary: Vec<_> = by_model.into_iter().collect();
model_summary.sort_by(|a, b| {
    b.1.1
        .partial_cmp(&a.1.1)
        .unwrap_or(std::cmp::Ordering::Equal)
});
for (model, (count, cost, passed, inp, out)) in &model_summary {
    let model_pass = if *count == 0 {
        0.0
    } else {
        *passed as f64 / *count as f64 * 100.0
    };
    println!(
        "  {model}: {count} runs, {}, {model_pass:.0}% pass",
        display_cost(*cost, *inp, *out)
    );
}
```

### Step 3: Update `print_learn_episodes` to use the helper

**Change 1**: Total cost line (lines 311-316).

For total cost across all episodes, we need the aggregate token counts:

**Before**:
```rust
let total_cost: f64 = episodes.iter().map(|e| f64::from(e.usage.cost_usd)).sum();
// ...
println!(
    "Episodes: {} recorded, {:.0}% success, ${:.2} total cost",
    episodes.len(),
    pass_rate,
    total_cost,
);
```

**After**:
```rust
let total_cost: f64 = episodes.iter().map(|e| e.usage.cost_usd).sum();
let total_ep_input: u64 = episodes.iter().map(|e| e.usage.input_tokens).sum();
let total_ep_output: u64 = episodes.iter().map(|e| e.usage.output_tokens).sum();
// ...
println!(
    "Episodes: {} recorded, {:.0}% success, {} total cost",
    episodes.len(),
    pass_rate,
    display_cost(total_cost, total_ep_input, total_ep_output),
);
```

Note: `Episode.usage` in `roko-learn` uses `f64` for `cost_usd` and `u64` for `input_tokens`/`output_tokens`, so `f64::from(e.usage.cost_usd)` was redundant anyway — use `e.usage.cost_usd` directly.

**Change 2**: Recent episodes display (lines 343-352).

**Before**:
```rust
println!(
    "    {} {} {} [{}] ${:.4}",
    ep.timestamp.format("%Y-%m-%d %H:%M"),
    ep.model,
    ep.task_id,
    status,
    ep.usage.cost_usd,
);
```

**After**:
```rust
println!(
    "    {} {} {} [{}] {}",
    ep.timestamp.format("%Y-%m-%d %H:%M"),
    ep.model,
    ep.task_id,
    status,
    display_cost_precise(ep.usage.cost_usd, ep.usage.input_tokens, ep.usage.output_tokens),
);
```

## Acceptance Criteria
- [ ] `display_cost` helper added to `learn.rs`
- [ ] `print_learn_efficiency` total cost: shows "unknown" instead of "$0.00" when all events have zero tokens and zero cost
- [ ] `print_learn_efficiency` per-model cost: shows "unknown" for zero-usage models
- [ ] `print_learn_episodes` total cost: shows "unknown" when aggregate tokens and cost are zero
- [ ] `print_learn_episodes` recent list: shows "unknown" instead of "$0.0000" for zero-usage episodes
- [ ] `print_learn_episodes` recent list: shows "$1.4200" style for non-zero cost
- [ ] No changes to stored values in JSONL files
- [ ] `cargo build -p roko-cli` succeeds
- [ ] `cargo clippy -p roko-cli --no-deps -- -D warnings` passes

## Verification
```bash
cargo build -p roko-cli
cargo clippy -p roko-cli --no-deps -- -D warnings
# Manual: run learn all against a workspace with no usage data:
# cargo run -p roko-cli -- learn all
# Output should show "unknown" instead of "$0.00" for costs
```

A unit test to add to the learn module (or a test file):
```rust
#[test]
fn display_cost_unknown_when_all_zero() {
    assert_eq!(display_cost(0.0, 0, 0), "unknown");
}

#[test]
fn display_cost_shows_zero_when_tokens_present() {
    assert_eq!(display_cost(0.0, 100, 50), "$0.00");
}

#[test]
fn display_cost_shows_formatted_value() {
    assert_eq!(display_cost(1.42, 100, 50), "$1.42");
}
```

## Do NOT
- Change stored values in JSONL files
- Make ALL "$0.00" display as "unknown" — `cost_usd == 0.0` with non-zero tokens means "genuinely free"
- Touch the dashboard frontend (`demo-app/`)
- Touch `crates/roko-serve/` routes — HTTP API is out of scope for this batch
- Change `status.rs` — it already handles `Option<f64>` cost correctly

## Evidence
- `crates/roko-cli/src/commands/learn.rs` lines 226-355 (efficiency and episode display)
- `crates/roko-cli/src/status.rs` lines 94-99 (status display — already uses Option, no change needed)
- `crates/roko-learn/src/efficiency.rs` lines 79-163 (`AgentEfficiencyEvent.cost_usd: f64`)
- `crates/roko-learn/src/episode_logger.rs` lines 121-145 (`Usage.cost_usd: f64`)
- Actual efficiency.jsonl sample: `{"cost_usd":0.0,"input_tokens":0,...}` — zeros everywhere before R5_A01

---

## Current Implementation (as written by implementation agent)

### `crates/roko-cli/src/commands/status.rs` — missing (should have been created)

### `crates/roko-cli/src/commands/learn.rs`

```rust
//! learn command handlers.
#![allow(unused_imports)]

use crate::*;

/// Format a cost value for human display.
///
/// Uses the heuristic: if cost is exactly `0.0` and both token counts are
/// zero, treat the usage as unknown because the provider did not report it.
fn display_cost(cost_usd: f64, input_tokens: u64, output_tokens: u64) -> String {
    if cost_usd == 0.0 && input_tokens == 0 && output_tokens == 0 {
        "unknown".to_string()
    } else {
        format!("${cost_usd:.2}")
    }
}

/// Format a cost value for recent-entry display with four decimal places.
fn display_cost_precise(cost_usd: f64, input_tokens: u64, output_tokens: u64) -> String {
    let display = display_cost(cost_usd, input_tokens, output_tokens);
    if display == "unknown" {
        display
    } else {
        format!("${cost_usd:.4}")
    }
}

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
        let Ok(event) =
            serde_json::from_str::<roko_learn::efficiency::AgentEfficiencyEvent>(trimmed)
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
            "{timestamp} model={model} task={task_id} plan={plan_id} {status} cost={}",
            display_cost_precise(event.cost_usd, event.input_tokens, event.output_tokens)
        ));
    }

    println!("Efficiency: {} events at {}", count, path.display());
    println!("  Range: {}", format_range(first_seen, last_seen));
    println!("  Latest: {}", latest.unwrap_or_else(|| "none".to_string()));
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
            "{} model={model} task={task_id} {status} cost={}",
            episode.timestamp.to_rfc3339(),
            display_cost_precise(
                episode.usage.cost_usd,
                episode.usage.input_tokens,
                episode.usage.output_tokens,
            )
        ));
    }

    println!("Episodes: {} entries at {}", count, path.display());
    println!("  Range: {}", format_range(first_seen, last_seen));
    println!("  Latest: {}", latest.unwrap_or_else(|| "none".to_string()));
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
    if trimmed.is_empty() {
        "unknown"
    } else {
        trimmed
    }
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

#[cfg(test)]
mod tests {
    use super::{display_cost, display_cost_precise};

    #[test]
    fn display_cost_unknown_when_all_zero() {
        assert_eq!(display_cost(0.0, 0, 0), "unknown");
    }

    #[test]
    fn display_cost_shows_zero_when_tokens_present() {
        assert_eq!(display_cost(0.0, 100, 50), "$0.00");
    }

    #[test]
    fn display_cost_shows_formatted_value() {
        assert_eq!(display_cost(1.42, 100, 50), "$1.42");
    }

    #[test]
    fn display_cost_precise_unknown_when_all_zero() {
        assert_eq!(display_cost_precise(0.0, 0, 0), "unknown");
    }

    #[test]
    fn display_cost_precise_shows_zero_when_tokens_present() {
        assert_eq!(display_cost_precise(0.0, 100, 50), "$0.0000");
    }

    #[test]
    fn display_cost_precise_shows_formatted_value() {
        assert_eq!(display_cost_precise(1.42, 100, 50), "$1.4200");
    }
}
```

### `crates/roko-serve/src/routes/` — missing (should have been created)

---

## Read-Only Context (do not modify)

### `crates/roko-agent/src/usage.rs`

```rust
//! Compatibility re-export for shared usage metrics.
//!
//! `Usage` remains the legacy flat counter shape from `roko-core`.
//! `UsageObservation` is the canonical telemetry-facing shape that can
//! distinguish "not reported" from zero.

use serde::{Deserialize, Serialize};

pub use roko_core::chat_types::Usage;

/// Canonical usage observation for agent attempts and model calls.
///
/// Numeric fields are optional so unknown values stay unknown rather than
/// collapsing to zero.
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
#[serde(default)]
pub struct UsageObservation {
    pub input_tokens: Option<u64>,
    pub output_tokens: Option<u64>,
    #[serde(alias = "cache_create_tokens")]
    pub cache_creation_tokens: Option<u64>,
    pub cache_read_tokens: Option<u64>,
    pub cost_usd: Option<f64>,
    #[serde(default)]
    pub source: UsageSource,
    pub model: Option<String>,
    pub wall_ms: u64,
}

/// Provenance for a usage observation.
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
pub enum UsageSource {
    /// Provider-reported usage from the backend response.
    ProviderReported,
    /// Estimated from local accounting.
    Estimated,
    /// Source not known.
    #[default]
    Unknown,
}

impl From<Usage> for UsageObservation {
    fn from(usage: Usage) -> Self {
        Self {
            input_tokens: Some(u64::from(usage.input_tokens)),
            output_tokens: Some(u64::from(usage.output_tokens)),
            cache_creation_tokens: Some(u64::from(usage.cache_create_tokens)),
            cache_read_tokens: Some(u64::from(usage.cache_read_tokens)),
            cost_usd: Some(f64::from(usage.cost_usd)),
            source: UsageSource::Unknown,
            model: None,
            wall_ms: usage.wall_ms,
        }
    }
}

impl From<UsageObservation> for Usage {
    fn from(observation: UsageObservation) -> Self {
        let clamp_u32 = |value: Option<u64>| match value {
            Some(value) => u32::try_from(value).unwrap_or(u32::MAX),
            None => 0,
        };

        Self {
            input_tokens: clamp_u32(observation.input_tokens),
            output_tokens: clamp_u32(observation.output_tokens),
            cache_read_tokens: clamp_u32(observation.cache_read_tokens),
            cache_create_tokens: clamp_u32(observation.cache_creation_tokens),
            cost_usd: observation.cost_usd.map_or(0.0, |value| value as f32),
            wall_ms: observation.wall_ms,
        }
    }
}
```

---

## Verification Commands

Run these and fix any failures:
```bash
cargo check -p roko-cli
cargo clippy -p roko-cli --no-deps -- -D warnings
cargo check -p roko-serve
cargo clippy -p roko-serve --no-deps -- -D warnings
```

## Do NOT

- Rewrite the entire implementation from scratch
- Add features not in the original prompt
- Modify files outside the write scope
- Skip running verification commands
