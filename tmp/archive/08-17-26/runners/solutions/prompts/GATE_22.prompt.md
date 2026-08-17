# GATE_22: Emit RuntimeEvents from GateService

## Tracker

- Issue tracker row: [`ISSUE-TRACKER.md#gate-22`](../ISSUE-TRACKER.md#gate-22)
- Source: `tmp/solutions/roko/tasks/04-GATE-PIPELINE.md` — Task 4.22
- Priority: **P2**
- Effort: 3 hours
- Depends on: `GATE_21` (source 4.21)

When this batch lands, the commit message MUST contain the trailer:

```
tracker: GATE_22 done
```

`bin/sync-tracker.py --apply` flips the corresponding `[ ]` to `[x]`.

## Problem

GateService runs gates silently -- no events are emitted during execution. The TUI and dashboard only see results after the full pipeline completes. Emitting per-gate events enables real-time progress display.

## Exact Changes

1. Add an event sink to GateService:
   ```rust
   pub struct GateService {
       adaptive: Option<Arc<Mutex<AdaptiveThresholds>>>,
       temperament: Temperament,
       event_sink: Option<tokio::sync::mpsc::Sender<RuntimeEvent>>,
       run_id: String,
   }
   ```
2. Add builder method:
   ```rust
   pub fn with_event_sink(mut self, sink: mpsc::Sender<RuntimeEvent>, run_id: String) -> Self {
       self.event_sink = Some(sink);
       self.run_id = run_id;
       self
   }
   ```
3. Emit events in the `run_gates()` loop:
   ```rust
   // Before gate execution:
   self.emit(RuntimeEvent::GateStarted {
       run_id: self.run_id.clone(),
       gate_name: gate_name.clone(),
       rung,
   });

   // After gate execution:
   if verdict.passed {
       self.emit(RuntimeEvent::GatePassed { ... });
   } else {
       self.emit(RuntimeEvent::GateFailed { ... });
   }

   // On skip:
   self.emit(RuntimeEvent::GateSkipped { ... });
   ```
4. After the full pipeline:
   ```rust
   self.emit(RuntimeEvent::GatePipelineCompleted { ... });
   ```
5. Add a non-blocking emit helper:
   ```rust
   fn emit(&self, event: RuntimeEvent) {
       if let Some(sink) = &self.event_sink {
           let _ = sink.try_send(event);
       }
   }
   ```

## Write Scope

- `crates/roko-gate/src/gate_service.rs`

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

- [ ] Events are emitted for each gate start/pass/fail/skip
- [ ] Pipeline completion event includes aggregate statistics
- [ ] When no event sink is configured, no events are emitted (no-op)

## Verify Recipe

```bash
# Spot-check with ripgrep / git on the touched files.
# Do NOT run cargo — the merge-back pipeline does that.
git diff --stat
```

Then check the commit-message trailer:

```bash
git log -1 --format=%B | rg "^tracker: GATE_22 done"
```

## Acceptance Criteria

- All Verify checkboxes pass on inspection.
- Events are emitted for each gate start/pass/fail/skip
- Pipeline completion event includes aggregate statistics
- When no event sink is configured, no events are emitted (no-op)
- No files outside the Write Scope are modified.
- Commit message contains `tracker: GATE_22 done` trailer.
- Pre-commit pipeline (fmt + clippy + test) passes when run by the merge-back stage.

## Do NOT

- Compile or run tests inside the batch (`cargo check/test/clippy/build`).
- Touch files outside the Write Scope above.
- Bundle this batch with another tracker row, even if both touch the same file.
- Add `unwrap()`, `panic!()`, or `unreachable!()` to changed code.
- Add a second dispatch / prompt-assembly / chat-state path. See
  `context-pack/00-RULES.md` §"Universal anti-patterns".
- Refactor neighbours opportunistically. File a separate tracker row.
