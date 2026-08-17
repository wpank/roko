# XCUT_09: Gate Results as Structured Compliance Events

## Tracker

- Issue tracker row: [`ISSUE-TRACKER.md#xcut-09`](../ISSUE-TRACKER.md#xcut-09)
- Source: `tmp/solutions/roko/tasks/19-CROSS-CUTTING.md` — Task 19.9
- Priority: **P5**
- Effort: 3 hours
- Depends on: `XCUT_08` (source 19.8)

When this batch lands, the commit message MUST contain the trailer:

```
tracker: XCUT_09 done
```

`bin/sync-tracker.py --apply` flips the corresponding `[ ]` to `[x]`.

## Problem

Gate results are logged to JSONL but not emitted as structured events. `RuntimeEvent` (line 56 of `runtime_event.rs`) has `GateStarted`, `GatePassed`, and `GateFailed` variants but no compliance-specific event with the detail needed for SIEM/GRC integration. The gate pipeline produces structured `GateVerdict` results internally but flattens them to pass/fail strings in the event.

## Exact Changes

1. Add `RuntimeEvent::GateCompliance { gate_name, rung, verdict, detail, duration_ms, agent_id, task_id }` variant.
2. In `GateService::run_gates()`, after each gate execution emit a `GateCompliance` event to the event bus.
3. If OTel is configured (Task 19.8), also emit an OTel span per gate with attributes: `roko.gate.name`, `roko.gate.rung`, `roko.gate.verdict`, `roko.gate.duration_ms`.
4. Add `[gates] compliance_events = true` config flag (default false).
5. When enabled, gate events flow through EventBus to SSE/WebSocket for external consumers.
6. Update `RuntimeEvent::run_id()` and `RuntimeEvent::kind()` match arms for the new variant.

## Write Scope

- `crates/roko-gate/src/gate_service.rs`
- `crates/roko-core/src/runtime_event.rs`

## Read-Only Context

The following files contain context that informs this change but **must
not** be modified by this batch. If you find yourself wanting to edit
them, file a follow-up tracker row instead.

- `tmp/runners/solutions/context-pack/00-RULES.md`
- `tmp/runners/solutions/context-pack/02-FILE-INVENTORY.md`
- `tmp/solutions/roko/tasks/19-CROSS-CUTTING.md` (the full task list this came from)

## Verification Checklist

These criteria come straight from the source task. Tick each one before
opening the merge:

- [ ] With `compliance_events = true`, each gate execution produces a `GateCompliance` RuntimeEvent
- [ ] Events are visible via SSE at `/api/events`
- [ ] Without the flag, no extra event overhead
- [ ] Gate events include the full verdict (pass/fail/skip) with detail

## Verify Recipe

```bash
# Spot-check with ripgrep / git on the touched files.
# Do NOT run cargo — the merge-back pipeline does that.
git diff --stat
```

Then check the commit-message trailer:

```bash
git log -1 --format=%B | rg "^tracker: XCUT_09 done"
```

## Acceptance Criteria

- All Verify checkboxes pass on inspection.
- With `compliance_events = true`, each gate execution produces a `GateCompliance` RuntimeEvent
- Events are visible via SSE at `/api/events`
- Without the flag, no extra event overhead
- Gate events include the full verdict (pass/fail/skip) with detail
- No files outside the Write Scope are modified.
- Commit message contains `tracker: XCUT_09 done` trailer.
- Pre-commit pipeline (fmt + clippy + test) passes when run by the merge-back stage.

## Do NOT

- Compile or run tests inside the batch (`cargo check/test/clippy/build`).
- Touch files outside the Write Scope above.
- Bundle this batch with another tracker row, even if both touch the same file.
- Add `unwrap()`, `panic!()`, or `unreachable!()` to changed code.
- Add a second dispatch / prompt-assembly / chat-state path. See
  `context-pack/00-RULES.md` §"Universal anti-patterns".
- Refactor neighbours opportunistically. File a separate tracker row.
