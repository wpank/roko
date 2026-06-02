# W9-D: Observability & Instrumentation -- Pipeline-Wide Timing & Debug Logging

**Priority**: P0 -- without timing and debug logs, bottlenecks are invisible and unused code paths cannot be identified
**Effort**: 45 minutes
**Files to modify**: 5 files
**Dependencies**: None

## Problem

The roko pipeline has limited visibility into execution timing and flow. While some timing exists (factory init, task-level dispatch_ms/agent_ms/gate_ms), critical gaps remain:

1. **Plan run startup**: No logging of resolved config, provider, model, budget, or workspace state at run start. Operators cannot tell what configuration the runner actually used.
2. **Task dispatch chain**: Model selection reasoning and prompt size are logged at `debug!` level but not summarized at `info!` level. The dispatch_ms timing only fires if >50ms, hiding fast but still useful data points.
3. **Gate pipeline**: Gate rung start events have no logging of what commands will run. The stderr output is logged but the first 200 chars of stderr (useful for quick diagnosis) are not included in structured tracing.
4. **Cost accumulation**: Token counts flow through `agent_events.rs` at `debug!` level only. Cost roll-into-totals happens silently. No `info!`-level breadcrumb shows when cost is accumulated or what the running total is.
5. **Plan completion summary**: The final report prints to stderr but no structured `tracing::info!` captures the full per-task breakdown for log analysis.
6. **PRD pipeline**: Already has good phase timing (init/prompt/context/agent/post/learn/total). Needs enhancement for TOML retry visibility and extraction diagnostics.

## Root Cause

Instrumentation was added incrementally as features were wired. Each subsystem logs internally but there is no unified "observability layer" that provides a consistent timing+context breadcrumb trail across the full execution path. Key events that operators need to diagnose bottlenecks are either at `debug!` level (invisible by default) or missing entirely.

## Exact Code to Change

### File 1: `/Users/will/dev/nunchi/roko/roko/crates/roko-cli/src/runner/event_loop.rs`

#### Change 1.1: Add startup diagnostics after run config is finalized (line ~326-329)

The current code logs plan count and total tasks but not the resolved config settings that determine runtime behavior.

**Current code:**
```rust
    info!(
        plan_count = plans.len(),
        total_tasks, "starting runner v2 event loop"
    );
```

**Replace with:**
```rust
    info!(
        plan_count = plans.len(),
        total_tasks,
        model = %config.model,
        max_concurrent = config.max_concurrent_tasks,
        max_retries = config.max_retries,
        max_gate_rung = config.max_gate_rung,
        max_plan_usd = config.max_plan_usd,
        max_turn_usd = config.max_turn_usd,
        timeout_secs = config.timeout_secs,
        plan_timeout_secs = config.plan_timeout_secs,
        clippy_enabled = config.clippy_enabled,
        skip_tests = config.skip_tests,
        stream_to_stderr = config.stream_to_stderr,
        has_mcp_config = config.mcp_config.is_some(),
        has_cascade_router = config.cascade_router.is_some(),
        "starting runner v2 event loop"
    );
```

#### Change 1.2: Promote dispatch_ms logging to always fire (line ~991-997)

Currently, action dispatch timing only logs when >50ms. For observability, always log at `info!` for spawn actions and at `debug!` for fast actions.

**Current code:**
```rust
                    let dispatch_ms = t_dispatch.elapsed().as_millis() as u64;
                    if matches!(&action, ExecutorAction::SpawnAgent { .. }) {
                        ctx.state.last_dispatch_ms = dispatch_ms;
                    }
                    if dispatch_ms > 50 {
                        info!(action = %action_label, dispatch_ms, "action dispatched");
                    }
```

**Replace with:**
```rust
                    let dispatch_ms = t_dispatch.elapsed().as_millis() as u64;
                    if matches!(&action, ExecutorAction::SpawnAgent { .. }) {
                        ctx.state.last_dispatch_ms = dispatch_ms;
                        info!(action = %action_label, dispatch_ms, "agent action dispatched");
                    } else if dispatch_ms > 50 {
                        info!(action = %action_label, dispatch_ms, "action dispatched (slow)");
                    } else {
                        debug!(action = %action_label, dispatch_ms, "action dispatched");
                    }
```

#### Change 1.3: Add info-level model selection summary at dispatch (line ~2202-2217)

The model selection result and prompt diagnostics are only at `debug!` level. Add a concise `info!` summary.

**Current code:**
```rust
            let requested_model = dispatch_plan.model.slug.clone();
            let prompt_diagnostics = dispatch_plan.prompt.diagnostics.clone();
            ctx.tui
                .model_selected(plan_id, &task_id, &requested_model, selected_source);
            let system_prompt = dispatch_plan.prompt.system_prompt;
            let mut final_prompt = dispatch_plan.prompt.user_prompt;
            debug!(
                plan_id = %plan_id,
                task = %task_id,
                model = %requested_model,
                included_sections = ?dispatch_plan.prompt.diagnostics.included_sections,
                dropped_sections = ?dispatch_plan.prompt.diagnostics.dropped_sections,
                knowledge_ids = ?dispatch_plan.prompt.diagnostics.knowledge_ids,
                playbook_ids = ?dispatch_plan.prompt.diagnostics.playbook_ids,
                "dispatch prompt assembled"
            );
```

**Replace with:**
```rust
            let requested_model = dispatch_plan.model.slug.clone();
            let prompt_diagnostics = dispatch_plan.prompt.diagnostics.clone();
            ctx.tui
                .model_selected(plan_id, &task_id, &requested_model, selected_source);
            let system_prompt = dispatch_plan.prompt.system_prompt;
            let mut final_prompt = dispatch_plan.prompt.user_prompt;
            info!(
                plan_id = %plan_id,
                task = %task_id,
                model = %requested_model,
                source = selected_source,
                system_prompt_len = system_prompt.len(),
                user_prompt_len = final_prompt.len(),
                estimated_tokens = prompt_diagnostics.estimated_tokens,
                included_sections = prompt_diagnostics.included_sections.len(),
                dropped_sections = prompt_diagnostics.dropped_sections.len(),
                "dispatch: model selected, prompt assembled"
            );
            debug!(
                plan_id = %plan_id,
                task = %task_id,
                included_sections = ?dispatch_plan.prompt.diagnostics.included_sections,
                dropped_sections = ?dispatch_plan.prompt.diagnostics.dropped_sections,
                knowledge_ids = ?dispatch_plan.prompt.diagnostics.knowledge_ids,
                playbook_ids = ?dispatch_plan.prompt.diagnostics.playbook_ids,
                "dispatch prompt detail"
            );
```

#### Change 1.4: Add run completion summary with per-task breakdown (line ~1054-1071)

The terminal condition logs "all plans terminal" but not the summary. Add a structured summary.

**Current code:**
        if all_plans_terminal(&executor, &plans) {
            save_snapshot(
                config,
                &executor,
                &paths,
                &mut state,
                &merge_queue,
                &snapshot_writer,
            );
            let outcome = if build_report(&executor, &plans, &state).all_succeeded() {
                RunOutcome::Succeeded
            } else {
                RunOutcome::Failed
            };
            let event = build_run_completed_event(&executor, &plans, &state, outcome);
            emit_runner_event(&paths, &mut state, &tui, config, event);
            info!("all plans terminal — exiting event loop");
            break;
        }

**Replace with:**
```rust
        if all_plans_terminal(&executor, &plans) {
            save_snapshot(
                config,
                &executor,
                &paths,
                &mut state,
                &merge_queue,
                &snapshot_writer,
            );
            let final_report = build_report(&executor, &plans, &state);
            let outcome = if final_report.all_succeeded() {
                RunOutcome::Succeeded
            } else {
                RunOutcome::Failed
            };
            let event = build_run_completed_event(&executor, &plans, &state, outcome);
            emit_runner_event(&paths, &mut state, &tui, config, event);
            let cost_display = format!("{:.4}", final_report.total_cost_usd);
            info!(
                outcome = ?outcome,
                total_tasks = final_report.total_tasks,
                completed = final_report.tasks_completed,
                failed = final_report.tasks_failed,
                cost_usd = %cost_display,
                tokens_in = final_report.total_tokens_in,
                tokens_out = final_report.total_tokens_out,
                agent_calls = final_report.total_agent_calls,
                duration_secs = final_report.duration.as_secs(),
                "run complete — exiting event loop"
            );
            for plan_report in &final_report.plans {
                info!(
                    plan_id = %plan_report.plan_id,
                    completed = plan_report.completed,
                    tasks_done = plan_report.tasks_completed,
                    tasks_total = plan_report.tasks_total,
                    tasks_failed = plan_report.tasks_failed,
                    "plan summary"
                );
            }
            break;
        }
```

### File 2: `/Users/will/dev/nunchi/roko/roko/crates/roko-cli/src/runner/agent_events.rs`

#### Change 2.1: Add info-level cost accumulation logging in TurnCompleted handler (line ~168-194)

Token usage and cost are accumulated silently. Add `info!`-level logging when cost is recorded so operators can see cost per task in logs.

**Current code:**
```rust
        AgentEvent::TurnCompleted {
            session_id,
            total_cost_usd,
            num_turns: _,
            is_error,
        } => {
            state.agent_active = false;
            state.agent_turn_completed = true;
            if let Some(sid) = session_id {
                state.session_id = Some(sid.clone());
            }
            if let Some(cost) = total_cost_usd {
                // Use the authoritative cost from the result event.
                state.cost_usd = *cost;
            }
            if *is_error {
                state.agent_output.push_str("\n[agent error]\n");
            }
            let agent_id = agent_id_for_state(state);
            tui.agent_completed(&agent_id);
            debug!(
                task = %state.current_task,
                tokens_in = state.tokens_in,
                tokens_out = state.tokens_out,
                cost = state.cost_usd,
                "agent turn completed"
            );
```

**Replace with:**
```rust
        AgentEvent::TurnCompleted {
            session_id,
            total_cost_usd,
            num_turns: _,
            is_error,
        } => {
            state.agent_active = false;
            state.agent_turn_completed = true;
            if let Some(sid) = session_id {
                state.session_id = Some(sid.clone());
            }
            if let Some(cost) = total_cost_usd {
                // Use the authoritative cost from the result event.
                state.cost_usd = *cost;
            }
            if *is_error {
                state.agent_output.push_str("\n[agent error]\n");
            }
            let agent_id = agent_id_for_state(state);
            tui.agent_completed(&agent_id);
            let cost_display = format!("{:.4}", state.cost_usd);
            info!(
                task = %state.current_task,
                plan_id = %state.plan_id,
                tokens_in = state.tokens_in,
                tokens_out = state.tokens_out,
                cache_read = state.cache_read_tokens,
                cache_write = state.cache_write_tokens,
                cost_usd = %cost_display,
                model = %state.agent_model,
                is_error = *is_error,
                "agent turn completed"
            );
```

Note: This requires adding `info` to the import list at the top of the file.

#### Change 2.2: Update import to include `info` (line ~8)

**Current code:**
```rust
use tracing::debug;
```

**Replace with:**
```rust
use tracing::{debug, info};
```

### File 3: `/Users/will/dev/nunchi/roko/roko/crates/roko-cli/src/runner/gate_dispatch.rs`

#### Change 3.1: Add info-level rung start logging with verify step details (line ~53-66)

Currently the gate starts silently. Add a log line showing what the gate will run.

**Current code:**
```rust
        let start = Instant::now();
        let signal = gate_signal(&plan_id, &task_id, rung, &workdir);
        let ctx = roko_core::Context::now();
        let limit = Duration::from_secs(timeout_secs.max(1));

        let workdir_for_run = workdir.clone();
        let run = async {
            let inputs = RungExecutionInputs::default();
            let config = RungExecutionConfig {
                source_roots: Some(vec![workdir_for_run]),
                ..Default::default()
            };

            let mut verdicts = run_rung(&signal, &ctx, rung, &inputs, &config).await;
```

**Replace with:**
```rust
        let start = Instant::now();
        let signal = gate_signal(&plan_id, &task_id, rung, &workdir);
        let ctx = roko_core::Context::now();
        let limit = Duration::from_secs(timeout_secs.max(1));

        info!(
            plan_id = %plan_id,
            task_id = %task_id,
            rung,
            timeout_secs,
            verify_step_count = verify_steps.len(),
            "gate rung starting"
        );

        let workdir_for_run = workdir.clone();
        let run = async {
            let inputs = RungExecutionInputs::default();
            let config = RungExecutionConfig {
                source_roots: Some(vec![workdir_for_run]),
                ..Default::default()
            };

            let mut verdicts = run_rung(&signal, &ctx, rung, &inputs, &config).await;
```

#### Change 3.2: Enhance gate completion logging with stderr preview (line ~103-110)

Add stderr/output preview to the existing completion log so operators can see gate failures without digging into event files.

**Current code:**
```rust
        info!(
            plan_id = %plan_id,
            task_id = %task_id,
            rung,
            passed,
            duration_ms,
            "gate completed"
        );
```

**Replace with:**
```rust
        let output_preview: String = output.chars().take(200).collect();
        let verdict_names: Vec<&str> = summaries.iter().map(|v| v.gate_name.as_str()).collect();
        info!(
            plan_id = %plan_id,
            task_id = %task_id,
            rung,
            passed,
            duration_ms,
            verdict_count = summaries.len(),
            verdicts = ?verdict_names,
            output_preview = %output_preview,
            "gate completed"
        );
```

Note: Move the `output_preview` and `verdict_names` computation before the `info!` call. The `output` variable is already computed at line 88, and `summaries` at line 91.

#### Change 3.3: Add verify step logging in `run_verify_steps` (line ~267-281)

Currently verify steps run silently. Add per-step timing.

**Current code:**
```rust
async fn run_verify_steps(
    signal: &Engram,
    ctx: &roko_core::Context,
    task_id: &str,
    verify_steps: Vec<VerifyStep>,
) -> Vec<Verdict> {
    let mut verdicts = Vec::new();
    for step in verify_steps {
        let gate = ShellGate::new("sh", vec!["-c".into(), step.command.clone()])
            .with_name(format!("task-verify:{}:{}", task_id, step.phase))
            .with_timeout_ms(step.timeout_ms);
        verdicts.push(gate.verify(signal, ctx).await);
    }
    verdicts
}
```

**Replace with:**
```rust
async fn run_verify_steps(
    signal: &Engram,
    ctx: &roko_core::Context,
    task_id: &str,
    verify_steps: Vec<VerifyStep>,
) -> Vec<Verdict> {
    let mut verdicts = Vec::new();
    for (i, step) in verify_steps.iter().enumerate() {
        let step_start = Instant::now();
        let gate = ShellGate::new("sh", vec!["-c".into(), step.command.clone()])
            .with_name(format!("task-verify:{}:{}", task_id, step.phase))
            .with_timeout_ms(step.timeout_ms);
        let verdict = gate.verify(signal, ctx).await;
        info!(
            task_id = %task_id,
            step = i + 1,
            total_steps = verify_steps.len(),
            phase = %step.phase,
            command = %step.command,
            passed = verdict.passed,
            elapsed_ms = step_start.elapsed().as_millis() as u64,
            "verify step completed"
        );
        verdicts.push(verdict);
    }
    verdicts
}
```

### File 4: `/Users/will/dev/nunchi/roko/roko/crates/roko-cli/src/commands/plan.rs`

#### Change 4.1: Add startup diagnostics logging after config is loaded (line ~546-548)

The setup timing is computed but only printed to stderr. Add structured tracing for log file analysis.

**Current code:**
```rust
            let setup_ms = t_setup.elapsed().as_millis();
            let v2_report =
                roko_cli::runner::event_loop::run(plans, &run_config, &state_hub, cancel).await?;
```

**Replace with:**
```rust
            let setup_ms = t_setup.elapsed().as_millis();
            tracing::info!(
                setup_ms,
                plan_count,
                total_tasks,
                default_model = %roko_config.agent.default_model,
                max_concurrent_tasks,
                max_retries = run_config.max_retries,
                max_gate_rung = run_config.max_gate_rung,
                max_plan_usd = %format!("{:.2}", run_config.max_plan_usd),
                max_turn_usd = %format!("{:.2}", run_config.max_turn_usd),
                clippy_enabled = run_config.clippy_enabled,
                skip_tests = run_config.skip_tests,
                plans_dir = %resolved_plans_dir.display(),
                "plan run: setup complete, entering event loop"
            );
            let v2_report =
                roko_cli::runner::event_loop::run(plans, &run_config, &state_hub, cancel).await?;
```

### File 5: `/Users/will/dev/nunchi/roko/roko/crates/roko-cli/src/prd.rs`

#### Change 5.1: Add TOML retry timing to the retry loop (line ~1128-1177)

Each retry attempt currently logs a `warn!` but not the elapsed time. Add timing around each retry.

**Current code:**
```rust
        // Retry up to 2 times on extraction/validation failure.
        if validated_toml.is_err() {
            let max_retries = 2u32;
            for attempt in 1..=max_retries {
                tracing::warn!(
                    attempt,
                    err = validated_toml.as_ref().unwrap_err().as_str(),
                    "prd plan: TOML extraction/validation failed, retrying"
                );
```

**Replace with:**
```rust
        // Retry up to 2 times on extraction/validation failure.
        if validated_toml.is_err() {
            let max_retries = 2u32;
            for attempt in 1..=max_retries {
                let t_retry = Instant::now();
                tracing::warn!(
                    attempt,
                    max_retries,
                    err = validated_toml.as_ref().unwrap_err().as_str(),
                    "prd plan: TOML extraction/validation failed, retrying"
                );
```

Then, after the retry result match block (line ~1161-1177), add retry timing:

**Current code (end of retry match block):**
```rust
                match retry_result {
                    Ok((0, retry_output)) if !retry_output.trim().is_empty() => {
                        validated_toml = try_extract_and_validate(&retry_output);
                        if validated_toml.is_ok() {
                            tracing::info!(attempt, "prd plan: retry succeeded");
                            eprintln!("✅ Retry {attempt} succeeded");
                            break;
                        }
                    }
                    Ok((code, _)) => {
                        tracing::warn!(attempt, code, "prd plan: retry agent failed");
                    }
                    Err(err) => {
                        tracing::warn!(attempt, %err, "prd plan: retry agent error");
                    }
                }
```

**Replace with:**
```rust
                match retry_result {
                    Ok((0, retry_output)) if !retry_output.trim().is_empty() => {
                        validated_toml = try_extract_and_validate(&retry_output);
                        if validated_toml.is_ok() {
                            tracing::info!(
                                attempt,
                                retry_ms = t_retry.elapsed().as_millis() as u64,
                                "prd plan: retry succeeded"
                            );
                            eprintln!("✅ Retry {attempt} succeeded");
                            break;
                        }
                    }
                    Ok((code, _)) => {
                        tracing::warn!(
                            attempt,
                            code,
                            retry_ms = t_retry.elapsed().as_millis() as u64,
                            "prd plan: retry agent failed"
                        );
                    }
                    Err(err) => {
                        tracing::warn!(
                            attempt,
                            %err,
                            retry_ms = t_retry.elapsed().as_millis() as u64,
                            "prd plan: retry agent error"
                        );
                    }
                }
```

#### Change 5.2: Add extraction-phase timing before post-processing (line ~1231)

After the TOML is validated and written, add a timing log for the extraction/write phase.

**Important**: The file already has `let post_ms = t_phase.elapsed().as_millis();` at line 1306 and a `tracing::info!` at lines 1308-1314 that logs all phase timings including `post_ms`. Change 5.2 adds an INTERMEDIATE timing point between the agent phase and the post-processing phase so you can see how long extraction + write took separately from post-processing.

**Current code (after the tasks.toml write, line ~1231):**
```rust
        let generated_changed = dry_run_fs::changed_tasks_files(&plans_root, &tasks_before);

        if !dry_run {
            if let Err(e) = regenerate_old_format_plans(
```

**Replace with:**
```rust
        let extraction_ms = t_phase.elapsed().as_millis();
        tracing::info!(
            extraction_and_write_ms = extraction_ms,
            "prd plan: TOML extracted and written"
        );
        let t_phase = Instant::now();
        let generated_changed = dry_run_fs::changed_tasks_files(&plans_root, &tasks_before);

        if !dry_run {
            if let Err(e) = regenerate_old_format_plans(
```

**Note on variable naming**: This intentionally shadows `t_phase` to reset the timer. The existing `let post_ms = t_phase.elapsed().as_millis();` at line 1306 will now measure only the post-processing time (old format regen + validation report + stats) rather than extraction+write+postprocessing combined. The `extraction_ms` variable avoids conflicting with the existing `post_ms` name.

## Verification

After applying all changes, run:

```bash
# 1. Build
cargo build --workspace 2>&1 | tail -5

# 2. Lint
cargo clippy --workspace --no-deps -- -D warnings 2>&1 | tail -5

# 3. Test
cargo test --workspace 2>&1 | tail -5

# 4. Manual verification: run a plan with RUST_LOG=info and verify timing appears
RUST_LOG=info cargo run -p roko-cli -- plan run plans/ --dry-run 2>&1 | grep -E "elapsed|timing|completed|dispatched|starting"

# 5. Verify PRD timing shows in logs
RUST_LOG=info cargo run -p roko-cli -- prd plan some-slug --dry-run 2>&1 | grep -E "phase timing|retry|extraction"
```

## What to look for in logs after this change

With `RUST_LOG=info` (the default for production runs), you will now see:

```
INFO starting runner v2 event loop plan_count=1 total_tasks=5 model=claude-sonnet-4-20250514 max_concurrent=4 ...
INFO plan run: setup complete, entering event loop setup_ms=234 plan_count=1 total_tasks=5 ...
INFO gate rung starting plan_id=my-plan task_id=T1 rung=0 timeout_secs=120 verify_step_count=2
INFO verify step completed task_id=T1 step=1 total_steps=2 phase=build command="cargo check" passed=true elapsed_ms=4523
INFO gate completed plan_id=my-plan task_id=T1 rung=0 passed=true duration_ms=5102 verdict_count=2 output_preview="..."
INFO dispatch: model selected, prompt assembled plan_id=my-plan task=T2 model=claude-sonnet-4-20250514 source=dispatcher system_prompt_len=4200 user_prompt_len=1800 estimated_tokens=2400
INFO agent action dispatched action=my-plan/T2 dispatch_ms=89
INFO agent turn completed task=T2 plan_id=my-plan tokens_in=2400 tokens_out=1200 cost_usd=0.0180 model=claude-sonnet-4-20250514
INFO run complete -- exiting event loop outcome=Succeeded total_tasks=5 completed=5 failed=0 cost_usd=0.0850 duration_secs=45
INFO plan summary plan_id=my-plan completed=true tasks_done=5 tasks_total=5
```

## Agent Prompt

```
You are implementing observability instrumentation for the roko pipeline. This is a purely additive change -- you are ONLY adding tracing::info! and tracing::debug! log lines and timing computations. You are NOT changing any control flow, data structures, or business logic.

Apply each change from the batch file exactly:
1. In event_loop.rs: enhance the startup info! log with config fields, promote dispatch timing, add model selection info!, add run completion summary
2. In agent_events.rs: promote TurnCompleted logging from debug! to info! with cost fields, add info to imports
3. In gate_dispatch.rs: add gate rung start logging, enhance gate completion with stderr preview, add per-step verify timing
4. In commands/plan.rs: add startup diagnostics tracing::info! before entering event loop
5. In prd.rs: add retry timing around TOML retry loop, add post-processing phase timing

For every change: the tracing macro call is the ONLY thing changing. No logic changes. No new types. No new dependencies. Just adding/promoting log lines and wrapping operations with Instant::now() / .elapsed().

After applying, run:
  cargo clippy --workspace --no-deps -- -D warnings
  cargo test --workspace
```

## Commit

```
feat: add comprehensive observability instrumentation across pipeline

Add info-level timing and diagnostic logging to plan run, agent dispatch,
gate pipeline, cost tracking, and PRD generation. Enables operators to
see full execution paths, timing breakdowns, and bottleneck identification
from standard log output (RUST_LOG=info).
```

## Checklist

- [ ] event_loop.rs: startup info! includes model, budget, concurrency, gate config
- [ ] event_loop.rs: dispatch_ms always logged (info for spawn, debug for others)
- [ ] event_loop.rs: model selection logged at info with prompt sizes
- [ ] event_loop.rs: run completion summary with per-plan breakdown
- [ ] agent_events.rs: TurnCompleted logged at info with cost/tokens/model
- [ ] agent_events.rs: `use tracing::{debug, info}` import
- [ ] gate_dispatch.rs: gate rung start logged with verify step count
- [ ] gate_dispatch.rs: gate completion includes output preview and verdict names
- [ ] gate_dispatch.rs: per-step verify timing logged
- [ ] commands/plan.rs: startup diagnostics logged before event loop entry
- [ ] prd.rs: TOML retry timing logged per attempt
- [ ] prd.rs: post-processing phase timing logged
- [ ] cargo clippy clean
- [ ] cargo test passes

## Audit Status

Audited: 2026-05-05. 1 issue fixed -- Change 5.2 variable naming corrected from `post_ms` to `extraction_ms` to avoid shadowing conflict with existing `post_ms` at prd.rs:1306; added explanatory note about `t_phase` shadowing intent
