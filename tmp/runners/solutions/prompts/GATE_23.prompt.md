# GATE_23: Consume gate events in TUI bridge

## Tracker

- Issue tracker row: [`ISSUE-TRACKER.md#gate-23`](../ISSUE-TRACKER.md#gate-23)
- Source: `tmp/solutions/roko/tasks/04-GATE-PIPELINE.md` — Task 4.23
- Priority: **P2**
- Effort: 3 hours
- Depends on: `GATE_22` (source 4.22)

When this batch lands, the commit message MUST contain the trailer:

```
tracker: GATE_23 done
```

`bin/sync-tracker.py --apply` flips the corresponding `[ ]` to `[x]`.

## Problem

The TUI's verdicts tab (`crates/roko-cli/src/tui/verdicts.rs:85` `VerdictsAggregator`) currently reads from the substrate after facts. With RuntimeEvent gate events, the TUI can show real-time gate progress.

The TUI bridge (`crates/roko-cli/src/runner/tui_bridge.rs`) translates between runtime events and StateHub dashboard events.

## Exact Changes

1. In `tui_bridge.rs`, handle the new gate RuntimeEvent variants:
   ```rust
   RuntimeEvent::GateStarted { gate_name, .. } => {
       state_hub.update_gate_status(&gate_name, "running");
   }
   RuntimeEvent::GatePassed { gate_name, duration_ms, .. } => {
       state_hub.update_gate_status(&gate_name, "passed");
       state_hub.update_gate_duration(&gate_name, duration_ms);
   }
   RuntimeEvent::GateFailed { gate_name, output, duration_ms, .. } => {
       state_hub.update_gate_status(&gate_name, "failed");
       state_hub.update_gate_detail(&gate_name, &output);
   }
   RuntimeEvent::GateSkipped { gate_name, reason, .. } => {
       state_hub.update_gate_status(&gate_name, "skipped");
       state_hub.update_gate_detail(&gate_name, &reason);
   }
   ```
2. In the verdicts tab, render real-time gate status from StateHub events.
3. Show SPC alerts and joint anomaly warnings prominently in the TUI.

## Write Scope

- `crates/roko-cli/src/runner/tui_bridge.rs`
- `crates/roko-cli/src/tui/verdicts.rs`

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

- [ ] Gate progress is visible in TUI during execution (not just after completion)
- [ ] Skipped gates show reason
- [ ] SPC alerts are surfaced in the TUI

## Verify Recipe

```bash
# Spot-check with ripgrep / git on the touched files.
# Do NOT run cargo — the merge-back pipeline does that.
git diff --stat
```

Then check the commit-message trailer:

```bash
git log -1 --format=%B | rg "^tracker: GATE_23 done"
```

## Acceptance Criteria

- All Verify checkboxes pass on inspection.
- Gate progress is visible in TUI during execution (not just after completion)
- Skipped gates show reason
- SPC alerts are surfaced in the TUI
- No files outside the Write Scope are modified.
- Commit message contains `tracker: GATE_23 done` trailer.
- Pre-commit pipeline (fmt + clippy + test) passes when run by the merge-back stage.

## Do NOT

- Compile or run tests inside the batch (`cargo check/test/clippy/build`).
- Touch files outside the Write Scope above.
- Bundle this batch with another tracker row, even if both touch the same file.
- Add `unwrap()`, `panic!()`, or `unreachable!()` to changed code.
- Add a second dispatch / prompt-assembly / chat-state path. See
  `context-pack/00-RULES.md` §"Universal anti-patterns".
- Refactor neighbours opportunistically. File a separate tracker row.
