# 18 — Proof Runs: Verify the Whole Stack

> Phase 7 of `tmp/workflow/UNIFIED-IMPLEMENTATION-PLAN.md`. Run after plans 01–17 land. The "did we actually fix it" suite.

---

## Status (2026-05-01)

**NOT EXECUTED** as a coordinated suite.

Some unit and integration tests exist (per audits). The 12-point proof matrix has not been run end-to-end against a single build.

---

## Goal

A single test suite — runnable as `cargo test --workspace --features proof` — that verifies the unified system meets its contract. Each proof maps to a feature claimed in `tmp/workflow/UNIFIED-IMPLEMENTATION-PLAN.md` § Phase 7.

When this suite passes, the unified runtime ships.

---

## Why This Exists

A multi-month migration touches every subsystem. Local tests pass but end-to-end behavior is unverified until each scenario is exercised against a single build of the binary. These proof runs are the acceptance gate.

---

## The 12 Proofs

Each proof is a `#[tokio::test(flavor = "multi_thread")]` in `crates/roko-cli/tests/proof_runs.rs`, gated by `#[cfg(feature = "proof")]`. Each must pass against the post-retirement (plan 12) build.

### Proof 7.1 — One-task implementation plan

```rust
#[tokio::test]
async fn proof_7_1_one_task_plan_full_loop() {
    let temp = test_workdir_with_plan(include_str!("fixtures/plans/one-task.toml"));
    let exit = run_roko(&["plan", "run", "plans/"], &temp).await?;
    assert_eq!(exit.code, 0);

    let episodes = read_episodes(&temp);
    assert_eq!(episodes.len(), 1);
    assert!(episodes[0].caller.starts_with("workflow"));

    let snapshot = read_run_state(&temp);
    assert_eq!(snapshot.completed_tasks.len(), 1);
    assert!(snapshot.task_fingerprints.contains_key("T1"));
}
```

Acceptance: `roko plan run` executes 1 task, episode written, run state persisted.

### Proof 7.2 — Multi-task dependency plan

```rust
#[tokio::test]
async fn proof_7_2_multi_task_dependency() {
    let temp = test_workdir_with_plan(include_str!("fixtures/plans/diamond.toml"));
    // diamond: A → {B, C} → D → E
    let exit = run_roko(&["plan", "run", "plans/", "--max-concurrent", "4"], &temp).await?;
    assert_eq!(exit.code, 0);

    let events = read_runtime_events(&temp);
    let order = events.iter().filter_map(|e| match &e.event {
        RuntimeEvent::TaskCompleted { task_id, .. } => Some(task_id.clone()),
        _ => None,
    }).collect::<Vec<_>>();

    // A must complete before B and C
    let pos = |id: &str| order.iter().position(|t| t == id).unwrap();
    assert!(pos("A") < pos("B"));
    assert!(pos("A") < pos("C"));
    assert!(pos("B") < pos("D"));
    assert!(pos("C") < pos("D"));
    assert!(pos("D") < pos("E"));

    // B and C should overlap (both Started before either Completed)
    let b_started = events.iter().position(|e| matches!(&e.event, RuntimeEvent::TaskStarted { task_id, .. } if task_id == "B")).unwrap();
    let c_started = events.iter().position(|e| matches!(&e.event, RuntimeEvent::TaskStarted { task_id, .. } if task_id == "C")).unwrap();
    let b_completed = events.iter().position(|e| matches!(&e.event, RuntimeEvent::TaskCompleted { task_id, .. } if task_id == "B")).unwrap();
    let c_completed = events.iter().position(|e| matches!(&e.event, RuntimeEvent::TaskCompleted { task_id, .. } if task_id == "C")).unwrap();
    assert!(b_started < c_completed && c_started < b_completed, "B and C must overlap");
}
```

Acceptance: DAG order correct + parallelism observed.

### Proof 7.3 — Gate failure → auto-fix

```rust
#[tokio::test]
async fn proof_7_3_gate_failure_autofix() {
    let temp = test_workdir_with_compile_error_seed();        // seeds src/lib.rs with `let x: i32 = "bad";`
    let exit = run_roko(&["run", "fix the type error", "--max-iterations", "3"], &temp).await?;
    assert_eq!(exit.code, 0);

    let events = read_runtime_events(&temp);
    let phases = events.iter().filter_map(|e| match &e.event {
        RuntimeEvent::PhaseTransition { from, to, .. } => Some((from.clone(), to.clone())),
        _ => None,
    }).collect::<Vec<_>>();

    // Should see: Implementing → Gating → AutoFixing → Gating → Complete
    assert!(phases.iter().any(|(_, to)| to == "AutoFixing"), "auto-fix triggered");
    let gate_failures = events.iter().filter(|e| matches!(&e.event, RuntimeEvent::GateFailed { .. })).count();
    let gate_passes = events.iter().filter(|e| matches!(&e.event, RuntimeEvent::GatePassed { .. })).count();
    assert!(gate_failures >= 1);
    assert!(gate_passes >= 1);
}
```

Acceptance: gate fails first time, autofix patches, gate passes second time.

### Proof 7.4 — Gate failure → replan after exhausted retries

```rust
#[tokio::test]
async fn proof_7_4_gate_failure_replan() {
    let temp = test_workdir_with_unfixable_seed();            // seed where autofix can't succeed in N iterations
    let exit = run_roko(&["run", "implement impossible thing", "--max-iterations", "2"], &temp).await?;
    // Either Halted or Replanned (depending on FailureClassifier rules)
    let events = read_runtime_events(&temp);
    let outcome = events.iter().rev().find_map(|e| match &e.event {
        RuntimeEvent::WorkflowCompleted { outcome, .. } => Some(outcome.clone()),
        _ => None,
    }).unwrap();

    let replan_events = events.iter().filter(|e| matches!(&e.event, RuntimeEvent::PhaseTransition { to, .. } if to == "AwaitingReplan")).count();
    assert!(replan_events > 0 || matches!(outcome, WorkflowOutcome::Halted { .. }));
}
```

### Proof 7.5 — Reviewer rejects, implementer retries with findings

```rust
#[tokio::test]
async fn proof_7_5_reviewer_rejection_loop() {
    // Use a stub reviewer that rejects the first attempt and approves the second
    let services = test_services_with_stub_reviewer_rejecting_first();
    let engine = WorkflowEngine::new(services);
    let report = engine.run(test_run_config_standard()).await?;
    assert!(report.success);

    let prompts = engine.captured_prompts();
    let reviewer_prompts = prompts.iter().filter(|p| p.role == Some("reviewer".into())).collect::<Vec<_>>();
    assert!(reviewer_prompts.len() >= 2);
    let second_implementer = prompts.iter().filter(|p| p.role == Some("implementer".into())).nth(1).unwrap();
    assert!(second_implementer.system.contains("Review Findings"));
}
```

### Proof 7.6 — Crash + resume

```rust
#[tokio::test]
async fn proof_7_6_crash_and_resume() {
    let temp = test_workdir_with_plan(include_str!("fixtures/plans/three-task.toml"));

    // Run 1: kill mid-task-2
    let mut child = spawn_roko(&["plan", "run", "plans/"], &temp).await?;
    wait_for_event(&child, RuntimeEvent::PhaseTransition { to: "Gating".into(), .. }, 10).await?;
    child.kill().await?;

    // Run 2: resume
    let exit = run_roko(&["plan", "run", "plans/", "--resume"], &temp).await?;
    assert_eq!(exit.code, 0);

    // Verify task 1 was NOT re-run
    let events = read_runtime_events(&temp);
    let task1_completions = events.iter().filter(|e| matches!(&e.event, RuntimeEvent::TaskCompleted { task_id, .. } if task_id == "T1")).count();
    assert_eq!(task1_completions, 1, "T1 should complete exactly once across resume");
}
```

### Proof 7.7 — Routing learns

```rust
#[tokio::test]
async fn proof_7_7_routing_learns_from_failures() {
    let temp = tempdir()?;
    // Force 5 failures on one model
    let services = test_services(temp.path()).with_model_simulator(|model| {
        if model == "claude-sonnet-4" { Err("simulated failure") } else { Ok(test_response()) }
    });

    for _ in 0..5 {
        let engine = WorkflowEngine::new(services.clone());
        let _ = engine.run(test_run_config()).await;
    }

    let router = read_cascade_router(&temp);
    let sonnet_share = router.arm_share("claude-sonnet-4");
    let opus_share = router.arm_share("claude-opus-4");
    assert!(opus_share > sonnet_share, "opus should be selected more after sonnet failures");
}
```

### Proof 7.8 — Knowledge reuse

```rust
#[tokio::test]
async fn proof_7_8_knowledge_reuse() {
    let temp = tempdir()?;
    let services = ServiceFactory::for_test(temp.path()).await?;

    // Run 1: implement add
    let _ = WorkflowEngine::new(services.clone()).run(run_config("implement add(a, b)")).await?;

    // Wait for distillation (or trigger sync mode)
    services.flush_distillation().await?;

    // Run 2: implement subtract — verify prompt mentions add
    let services2 = ServiceFactory::for_test(temp.path()).await?;
    let prompt = services2.assembler().assemble(PromptSpec {
        role: Some("implementer".into()),
        task: Some("implement subtract(a, b)".into()),
        ..Default::default()
    }).await?;

    assert!(prompt.diagnostics.knowledge_ids.iter().any(|id| id.starts_with("kn-")), "second run loaded knowledge");
    assert!(prompt.system.contains("add(") || prompt.system.contains("type check"), "knowledge appeared in prompt");
}
```

### Proof 7.9 — Provider matrix

```rust
#[tokio::test]
async fn proof_7_9_provider_matrix() {
    for provider in ["anthropic", "openai", "ollama"] {
        let temp = test_workdir_for_provider(provider);
        let exit = run_roko(&["run", "say hello"], &temp).await?;
        let outcome_event = last_runtime_event(&temp);
        match outcome_event {
            RuntimeEvent::WorkflowCompleted { outcome: WorkflowOutcome::Success { .. }, .. } => {},
            RuntimeEvent::AgentDoa { cause, .. } => {
                // Acceptable if provider not configured; must be classified
                eprintln!("provider {} unavailable: {:?}", provider, cause);
            },
            _ => panic!("provider {provider} produced unclassified outcome"),
        }
    }
}
```

### Proof 7.10 — HTTP query proof

```rust
#[tokio::test]
async fn proof_7_10_http_query() {
    let server = test_server_with_workflow_engine().await?;
    let plan_id = server.create_plan(test_plan_toml()).await?;
    let run_id = server.run_plan(&plan_id).await?;

    // wait for completion
    server.wait_for_run_complete(&run_id, Duration::from_secs(60)).await?;

    let events: Vec<RuntimeEventEnvelope> = server.client()
        .get(&format!("{}/api/runs/{run_id}/events", server.url()))
        .send().await?.json().await?;
    assert!(!events.is_empty());

    // events JSON matches .roko/events.jsonl
    let on_disk = read_events_jsonl(&server.workdir());
    assert_eq!(events.len(), on_disk.len());
}
```

### Proof 7.11 — ACP proof

```rust
#[tokio::test]
async fn proof_7_11_acp_workflow() {
    let acp = spawn_acp_server().await?;
    let session_id = acp.create_session().await?;
    acp.send_prompt(&session_id, "fix the typo in README", SessionConfig {
        workflow: "standard".into(),
        review_strictness: "quick".into(),
        max_iterations: 1,
        clippy_enabled: true,
        tests_enabled: true,
    }).await?;

    let updates = acp.collect_updates(&session_id, Duration::from_secs(60)).await?;
    let phases: Vec<&str> = updates.iter().filter_map(|u| u.phase()).collect();
    assert!(phases.contains(&"Implementing"));
    assert!(phases.contains(&"Gating"));
    assert!(phases.contains(&"Reviewing"));
    assert!(phases.contains(&"Committing"));
}
```

### Proof 7.12 — Single-prompt express

```rust
#[tokio::test]
async fn proof_7_12_single_prompt_express() {
    let temp = test_workdir_with_seed_file();
    let exit = run_roko(&["run", "add a comment to lib.rs"], &temp).await?;
    assert_eq!(exit.code, 0);

    let events = read_runtime_events(&temp);
    // Express path: Implementing → Gating → Committing → Complete
    let phases: Vec<String> = events.iter().filter_map(|e| match &e.event {
        RuntimeEvent::PhaseTransition { to, .. } => Some(to.clone()),
        _ => None,
    }).collect();
    assert_eq!(phases.first().map(|s| s.as_str()), Some("Implementing"));
    assert!(phases.contains(&"Gating".to_string()));
    assert!(phases.contains(&"Committing".to_string()));

    // Episode written
    let episodes = read_episodes(&temp);
    assert_eq!(episodes.len(), 1);
}
```

---

## Additional Cross-Cutting Proofs

These aren't in the original Phase 7 list but verify the cleanup work:

### Proof CC-1 — No bare claude spawns outside the adapter

```rust
#[test]
fn no_bare_claude_spawns() {
    let output = std::process::Command::new("rg")
        .args(&["Command::new\\(\"claude\"\\)", "crates/", "--type", "rust"])
        .output().unwrap();
    let lines: Vec<_> = String::from_utf8_lossy(&output.stdout).lines()
        .filter(|l| !l.contains("provider/claude_cli"))
        .filter(|l| !l.contains("test"))
        .collect();
    assert!(lines.is_empty(), "Found bare claude spawns:\n{}", lines.join("\n"));
}
```

### Proof CC-2 — One canonical FeedbackEvent

```rust
#[test]
fn one_feedback_event_enum() {
    let output = std::process::Command::new("rg")
        .args(&["pub enum FeedbackEvent", "crates/", "--type", "rust"])
        .output().unwrap();
    let count = String::from_utf8_lossy(&output.stdout).lines().count();
    assert_eq!(count, 1, "Expected 1 FeedbackEvent enum, found {count}");
}
```

### Proof CC-3 — Total LOC reduction

```rust
#[test]
fn total_loc_reduced_by_baseline() {
    let baseline = include_str!("baseline_loc.txt").trim().parse::<usize>().unwrap();
    let actual = total_rust_loc("crates/");
    assert!(actual < baseline.saturating_sub(100_000),
        "Expected at least 100K LOC reduction; baseline={baseline}, actual={actual}");
}
```

`baseline_loc.txt` is a snapshot taken at start of plan execution.

### Proof CC-4 — All required files present

```rust
#[test]
fn required_runtime_files_present() {
    for required in &[
        "crates/roko-runtime/src/persistence.rs",
        "crates/roko-runtime/src/failure_tracker.rs",
        "crates/roko-runtime/src/warning_store.rs",
        "crates/roko-learn/src/sinks/episodes.rs",
        "crates/roko-learn/src/sinks/threshold.rs",
        "crates/roko-learn/src/sinks/playbook.rs",
        "crates/roko-gate/src/llm_judge_oracle.rs",
        "crates/roko-agent/src/stderr_classifier.rs",
    ] {
        assert!(std::path::Path::new(required).exists(), "missing: {required}");
    }
}
```

### Proof CC-5 — All retired files absent

```rust
#[test]
fn retired_files_absent() {
    for removed in &[
        "crates/roko-cli/src/dispatch_direct.rs",
        "crates/roko-cli/src/orchestrate.rs",
        "crates/roko-cli/src/runner/event_loop.rs",
        "crates/roko-orchestrator/src/coordination.rs",
        "crates/roko-daimon/src/lib.rs",
        "crates/roko-compose/src/auction.rs",
        "crates/roko-learn/src/hdc.rs",
    ] {
        assert!(!std::path::Path::new(removed).exists(), "should be deleted: {removed}");
    }
}
```

---

## Anti-Patterns To Avoid

| ID | Manifestation | How to avoid |
|---|---|---|
| #6 Feedback afterthought | Tests that don't verify feedback wrote | Every proof checks `.roko/episodes.jsonl` or similar |
| #3 Build another runtime | Tests that mock the entire engine | Proofs run against the real binary |

---

## Things NOT To Do

1. **Don't make these unit tests.** They MUST run against the actual `roko` binary, in a real temp dir, exercising real subprocess spawns. Mocking defeats the purpose.
2. **Don't run them in `cargo test` by default.** Gate behind `--features proof` because each takes 10+ seconds (real LLM calls).
3. **Don't skip the cross-cutting proofs.** They catch regressions where a plan was "completed" but its retirement step wasn't done.
4. **Don't accept partial passes.** All 12 (+ cross-cutting) must pass before declaring done.
5. **Don't run against staging APIs.** Use `claude-haiku-4` or local Ollama for proof runs to keep cost manageable. Configurable via `[proof].cheap_model`.
6. **Don't share state between proofs.** Each gets a fresh `tempdir`. Avoid global state pollution.
7. **Don't run proofs in parallel without isolation.** They write to `.roko/` in the temp dir; if two share a dir they corrupt each other.

---

## Tests / Proof Criteria

```bash
cargo test --workspace --features proof -- --test-threads=1 --nocapture
# expected: all proofs pass; total runtime ~10-15 minutes
```

Reporting: produce `tmp/proof-runs/<timestamp>/report.md` with per-proof timing and outcome.

---

## Dependencies

This plan **requires** plans 01-17 to be substantially complete. Specifically:

- Plans 01-09 — for the engine to run end-to-end
- Plan 10 — for `RuntimeEvent` queries to work
- Plan 11 — for entry points to flow through the engine
- Plan 12 — for retirement-related proofs

This plan is the FINAL acceptance gate.

---

## Estimated Effort

**M.** ~1 week. Mostly authoring tests + tuning timeouts.

- 12 proofs × ~half day each = 6 days
- Cross-cutting proofs = 1 day
- Tuning, baseline LOC capture, CI integration = 1 day
