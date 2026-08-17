# DISP_28: Emit RouterDecision Events from ModelCallService

## Tracker

- Issue tracker row: [`ISSUE-TRACKER.md#disp-28`](../ISSUE-TRACKER.md#disp-28)
- Source: `tmp/solutions/roko/tasks/03-INFERENCE-DISPATCH.md` — Task 3.28
- Priority: **P2**
- Effort: 3 hours
- Depends on: `DISP_06` (source 3.6)

When this batch lands, the commit message MUST contain the trailer:

```
tracker: DISP_28 done
```

`bin/sync-tracker.py --apply` flips the corresponding `[ ]` to `[x]`.

## Problem

`ModelCallService` emits `RuntimeEvent` variants through `event_consumers`. Currently it emits basic call completion events. After CascadeRouter integration (Task 3.6), it should also emit routing decision events so dashboards and the TUI can show why a particular model was selected.

## Exact Changes

1. Add a `RuntimeEvent::RouterDecision` variant (or equivalent) to `roko-core`:
   ```rust
   RouterDecision {
       model: String,
       source: String,  // "cascade_router", "role_config", "cli_override", etc.
       candidates: Vec<(String, f64)>,  // (model, score) pairs
       reason: String,
       estimated_cost_usd: f64,
   }
   ```
2. In `ModelCallService::call()`, after model resolution, emit the decision:
   ```rust
   self.emit(RuntimeEvent::RouterDecision { ... });
   ```
3. If a routing observer is available, include candidate scores from the router
4. Include the `SelectionSource` label in the event

## Design Guidance

The event should be lightweight -- no large payloads. Include only what dashboards need: which model was chosen, why, what alternatives were considered, and expected cost. The TUI's router trace card can display this data.

## Write Scope

- `crates/roko-agent/src/model_call_service.rs`
- `crates/roko-core/src/lib.rs`

## Read-Only Context

The following files contain context that informs this change but **must
not** be modified by this batch. If you find yourself wanting to edit
them, file a follow-up tracker row instead.

- `tmp/runners/solutions/context-pack/00-RULES.md`
- `tmp/runners/solutions/context-pack/02-FILE-INVENTORY.md`
- `tmp/solutions/roko/tasks/03-INFERENCE-DISPATCH.md` (the full task list this came from)

## Verification Checklist

These criteria come straight from the source task. Tick each one before
opening the merge:

- [ ] A test that dispatches through `ModelCallService` with an event consumer receives a `RouterDecision` event
- [ ] The event includes model, source, and reason fields

## Verify Recipe

```bash
# Spot-check with ripgrep / git on the touched files.
# Do NOT run cargo — the merge-back pipeline does that.
git diff --stat
```

Then check the commit-message trailer:

```bash
git log -1 --format=%B | rg "^tracker: DISP_28 done"
```

## Acceptance Criteria

- All Verify checkboxes pass on inspection.
- A test that dispatches through `ModelCallService` with an event consumer receives a `RouterDecision` event
- The event includes model, source, and reason fields
- No files outside the Write Scope are modified.
- Commit message contains `tracker: DISP_28 done` trailer.
- Pre-commit pipeline (fmt + clippy + test) passes when run by the merge-back stage.

## Do NOT

- Compile or run tests inside the batch (`cargo check/test/clippy/build`).
- Touch files outside the Write Scope above.
- Bundle this batch with another tracker row, even if both touch the same file.
- Add `unwrap()`, `panic!()`, or `unreachable!()` to changed code.
- Add a second dispatch / prompt-assembly / chat-state path. See
  `context-pack/00-RULES.md` §"Universal anti-patterns".
- Refactor neighbours opportunistically. File a separate tracker row.
