# GATE_07: Align Runner v2 gate dispatch with GateService

## Tracker

- Issue tracker row: [`ISSUE-TRACKER.md#gate-07`](../ISSUE-TRACKER.md#gate-07)
- Source: `tmp/solutions/roko/tasks/04-GATE-PIPELINE.md` — Task 4.7
- Priority: **P1**
- Effort: 6 hours
- Depends on: `GATE_03` (source 4.3), `GATE_04` (source 4.4)

When this batch lands, the commit message MUST contain the trailer:

```
tracker: GATE_07 done
```

`bin/sync-tracker.py --apply` flips the corresponding `[ ]` to `[x]`.

## Problem

Runner v2's `gate_dispatch.rs` calls `rung_dispatch::run_rung()` directly, bypassing GateService entirely. It spawns gates as background tokio tasks with a semaphore for serialization (`GATE_SEMAPHORE`). The `spawn_gate()` function at line 29 constructs `RungExecutionInputs::default()` (all oracles missing -- triggering stub verdicts) and `RungExecutionConfig` with only `source_roots`.

The runner v2 event loop receives `GateCompletion` from the gate channel and uses `classify_failure_kind()` (line 287) which already calls `classify_gate_failure()` from roko-gate.

Migrating to GateService requires replacing the per-rung dispatch with a single GateService call that runs the full gate pipeline. The background task pattern can remain -- just wrap GateService::run_gates() instead of run_rung().

## Exact Changes

1. In `gate_dispatch.rs`, replace `spawn_gate()` internals:
   ```rust
   pub fn spawn_gate(
       plan_id: String,
       task_id: String,
       rung: u32,
       workdir: PathBuf,
       verify_steps: Vec<VerifyStep>,
       timeout_secs: u64,
       gate_tx: mpsc::Sender<GateCompletion>,
   ) {
       tokio::spawn(async move {
           let Ok(_permit) = gate_semaphore().acquire_owned().await else { return; };
           let start = Instant::now();

           // Use GateService instead of direct run_rung()
           let enabled = rung_gates_for_level(rung);
           let config = GateConfig {
               workdir: workdir.clone(),
               enabled_gates: enabled,
               shell_gates: vec![],
               max_rung: Some(rung as u8),
               complexity: None,
               prior_failures: None,
           };

           let svc = GateService::new();
           let limit = Duration::from_secs(timeout_secs.max(1));
           let report = match timeout(limit, svc.run_gates(config)).await {
               Ok(Ok(report)) => report,
               Ok(Err(e)) => { /* construct error GateCompletion */ },
               Err(_) => { /* timeout GateCompletion */ },
           };

           // Also run verify steps
           // ... (keep existing verify_steps logic)

           // Convert GateReport -> GateCompletion
           let completion = report_to_completion(plan_id, task_id, rung, report, start.elapsed());
           let _ = gate_tx.send(completion).await;
       });
   }
   ```
2. Add helper `rung_gates_for_level(rung: u32) -> Vec<String>` that maps rung level to gate names:
   ```rust
   fn rung_gates_for_level(rung: u32) -> Vec<String> {
       match rung {
           0 => vec!["compile".into()],
           1 => vec!["compile".into(), "clippy".into()],
           2 => vec!["compile".into(), "clippy".into(), "test".into()],
           _ => vec!["compile".into(), "clippy".into(), "test".into()],
       }
   }
   ```
3. Add helper `report_to_completion()` that converts `GateReport` fields to `GateCompletion` fields.
4. Update `event_loop.rs` gate completion handler to use `report.failure_classification.recommended_action` for retry routing (instead of re-classifying from raw output).
5. Keep the verify_steps logic (ShellGate for task-specific verify commands) -- this is runner-v2-specific and not part of GateService.

## Design Guidance

The semaphore pattern (`GATE_SEMAPHORE`) serializes gate execution to avoid concurrent cargo processes fighting over the target directory. This is correct and should remain even with GateService -- just wrap the GateService call inside the semaphore. The verify_steps pattern (ShellGate per task) complements GateService and should run after GateService's gates.

## Write Scope

- `crates/roko-cli/src/runner/gate_dispatch.rs`
- `crates/roko-cli/src/runner/event_loop.rs`

## Read-Only Context

The following files contain context that informs this change but **must
not** be modified by this batch. If you find yourself wanting to edit
them, file a follow-up tracker row instead.

- `tmp/runners/solutions/context-pack/00-RULES.md`
- `tmp/runners/solutions/context-pack/02-FILE-INVENTORY.md`
- `tmp/solutions/roko/tasks/04-GATE-PIPELINE.md` (the full task list this came from)

## Verification Checklist

These criteria come straight from the source task. Tick each one before
opening the merge:

- [ ] Runner v2 gate dispatch uses GateService internally
- [ ] `run_rung()` is no longer called directly from gate_dispatch.rs
- [ ] Verify steps still execute after GateService gates
- [ ] Gate completion includes feedback and classification from GateReport

## Verify Recipe

```bash
# Spot-check with ripgrep / git on the touched files.
# Do NOT run cargo — the merge-back pipeline does that.
git diff --stat
```

Then check the commit-message trailer:

```bash
git log -1 --format=%B | rg "^tracker: GATE_07 done"
```

## Acceptance Criteria

- All Verify checkboxes pass on inspection.
- Runner v2 gate dispatch uses GateService internally
- `run_rung()` is no longer called directly from gate_dispatch.rs
- Verify steps still execute after GateService gates
- Gate completion includes feedback and classification from GateReport
- No files outside the Write Scope are modified.
- Commit message contains `tracker: GATE_07 done` trailer.
- Pre-commit pipeline (fmt + clippy + test) passes when run by the merge-back stage.

## Do NOT

- Compile or run tests inside the batch (`cargo check/test/clippy/build`).
- Touch files outside the Write Scope above.
- Bundle this batch with another tracker row, even if both touch the same file.
- Add `unwrap()`, `panic!()`, or `unreachable!()` to changed code.
- Add a second dispatch / prompt-assembly / chat-state path. See
  `context-pack/00-RULES.md` §"Universal anti-patterns".
- Refactor neighbours opportunistically. File a separate tracker row.
