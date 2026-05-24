# M024 — Wire Bus into plan execution (lifecycle Pulses)

## Objective
Emit Cell/node lifecycle events as Pulses on the Bus during plan execution in orchestrate.rs. When a task starts, completes, or fails, publish corresponding Pulses using the topic taxonomy from M023. This enables downstream subscribers (TUI, calibration, logging) to react to execution events in real time.

## Scope
- Crates: `roko-cli`, `roko-core`, `roko-orchestrator`
- Files:
  - `crates/roko-cli/src/orchestrate.rs` (main execution loop)
  - `crates/roko-core/src/topics.rs` (topic constants from M023)
  - `crates/roko-core/src/pulse_bus.rs` (PulseBus impl)
- Phase ref: `tmp/unified-migration/02-PHASE-1-KERNEL.md` §1.2
- Spec ref: `tmp/unified/05-EXECUTION-ENGINE.md` §5 (Lifecycle Events)
- Depends on: M020 (Bus verified), M023 (topic constants)

## Steps
1. Find the plan execution entry point and task dispatch loop:
   ```bash
   grep -n 'fn run_plan\|fn execute_task\|fn dispatch_agent_with\|async fn.*plan.*run' crates/roko-cli/src/orchestrate.rs | head -10
   ```

2. Check if orchestrate.rs already has access to a Bus instance:
   ```bash
   grep -n 'Bus\|PulseBus\|bus\|event_bus' crates/roko-cli/src/orchestrate.rs | head -10
   ```

3. If no Bus is available, thread a `Arc<dyn Bus<Receiver = ...>>` or `Arc<PulseBus>` through the execution context. Add it as a field on whatever config/state struct orchestrate.rs uses.

4. At plan start, publish:
   ```rust
   bus.publish(Pulse::new(
       seq.fetch_add(1, Ordering::Relaxed),
       Topic::new(topics::FLOW_STARTED),
       Kind::Metric,
       Body::json(json!({ "plan": plan_name, "task_count": tasks.len() })),
   ))?;
   ```

5. At each task start:
   ```rust
   bus.publish(Pulse::new(seq_next(), Topic::new(topics::NODE_STARTED), Kind::Metric,
       Body::json(json!({ "task": task_id, "agent": agent_name }))))?;
   ```

6. At task completion (gate pass):
   ```rust
   bus.publish(Pulse::new(seq_next(), Topic::new(topics::NODE_COMPLETED), Kind::Metric,
       Body::json(json!({ "task": task_id, "verdict": "pass", "duration_ms": elapsed }))))?;
   ```

7. At task failure (gate fail):
   ```rust
   bus.publish(Pulse::new(seq_next(), Topic::new(topics::NODE_FAILED), Kind::Metric,
       Body::json(json!({ "task": task_id, "verdict": "fail", "reason": reason }))))?;
   ```

8. At plan completion:
   ```rust
   bus.publish(Pulse::new(seq_next(), Topic::new(topics::FLOW_COMPLETED), Kind::Metric,
       Body::json(json!({ "plan": plan_name, "passed": passed_count, "failed": failed_count }))))?;
   ```

9. Add a test that runs a mock plan execution and subscribes to `orchestration.*` on the Bus, verifying all lifecycle Pulses are received in order.

## Verification
```bash
cargo check -p roko-cli
cargo clippy -p roko-cli --no-deps -- -D warnings
# Verify Bus is used:
grep -c 'bus.publish' crates/roko-cli/src/orchestrate.rs
# Should be >= 4 (flow start, node start, node end, flow end)
```

## What NOT to do
- Do NOT make Bus required for plan execution — if no Bus is provided, execution should still work (just without Pulse emission). Use `Option<Arc<PulseBus>>` or a no-op fallback.
- Do NOT block execution waiting for subscribers — publish is fire-and-forget
- Do NOT emit Pulses for internal bookkeeping — only for events that external observers care about
- Do NOT change the existing EpisodeLogger or EfficiencyEvent paths — Pulses are complementary
