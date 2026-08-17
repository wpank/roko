# 31 - Runtime, Gate, Artifact, and Terminal Agent Packets

Purpose: break `24-runtime-gate-ledger-plan.md` into mechanical packets.
Use `28-agent-tasking-playbook.md` as the assignment template. These packets are about
truthful state representation first, behavior migration second.

Runtime/gate anti-patterns to avoid:

- Do not represent no-op, skipped, rejected, failed, invalid, or not-wired states as
  generic success.
- Do not infer final workflow truth by replaying display/event text when an effect
  already produced a typed outcome.
- Do not introduce another rung map, gate alias map, or artifact validity flag beside
  the shared owner.
- Do not remove event emission in packets that only change report source; UI compatibility
  still matters during migration.
- Do not add prompt scraping or terminal-output parsing as proof of command success.

## R1: Add CommitOutcome Without Behavior Change

Context files:

- `tmp/subsystem-audits/05-01/19-workflow-result-state-redesign.md`
- `tmp/subsystem-audits/05-01/24-runtime-gate-ledger-plan.md`

Write scope:

- `crates/roko-runtime/src/pipeline_state.rs` or a new runtime outcome module
- local tests

Mechanical steps:

1. Add `CommitOutcome` enum with `Created`, `NoChanges`, `Rejected`, `Failed`.
2. Add conversion from current `CommitDone/CommitFailed` shapes to `CommitOutcome`
   for compatibility.
3. Add tests that `Created` carries hash and `NoChanges` has no hash.

Do not:

- Do not change workflow behavior yet.
- Do not remove `CommitDone` yet.

Verification:

```bash
cargo test -p roko-runtime commit_outcome
cargo check -p roko-runtime
```

Acceptance:

- Type exists and can represent no-change without `"noop"`.

## R2: Replace Clean-Tree `"noop"` With CommitOutcome

Context files:

- `tmp/subsystem-audits/05-01/19-workflow-result-state-redesign.md`
- `tmp/subsystem-audits/05-01/24-runtime-gate-ledger-plan.md`

Write scope:

- `crates/roko-runtime/src/effect_driver.rs`
- `crates/roko-runtime/src/pipeline_state.rs`
- runtime tests

Mechanical steps:

1. Change `EffectDriver::commit` clean-tree branch to return `CommitOutcome::NoChanges`.
2. Change pipeline input to accept typed commit result.
3. Preserve old serialized report fields through compatibility conversion if needed.
4. Add a test for clean tree that fails if `"noop"` appears.

Do not:

- Do not change git add/commit mechanics beyond clean-tree outcome.
- Do not treat `NoChanges` as created commit.

Verification:

```bash
cargo test -p roko-runtime commit_no_changes
cargo check -p roko-runtime
rg '\"noop\"|CommitDone \\{ hash' crates/roko-runtime/src
```

Expected `rg`: no production matches after migration, or only compatibility tests.

Acceptance:

- Clean tree is typed as no changes, not a fake commit hash.

## R3: Add RunLedger Skeleton And Report Adapter

Context files:

- `tmp/subsystem-audits/05-01/19-workflow-result-state-redesign.md`
- `tmp/subsystem-audits/05-01/24-runtime-gate-ledger-plan.md`

Write scope:

- `crates/roko-runtime/src/run_ledger.rs`
- `crates/roko-runtime/src/lib.rs`
- tests

Mechanical steps:

1. Add `RunLedger` with run id, prompt, phase history, agent outcomes, gate runs,
   artifacts, commit, cancellation, checkpoint, event persistence health.
2. Add `RunLedger::to_report_compat(...) -> WorkflowRunReport`.
3. Write a unit test building a ledger with one agent outcome and verifying report
   model/provider/usage are derived from ledger fields.

Do not:

- Do not switch `WorkflowEngine` yet.
- Do not read event bus inside `RunLedger`.

Verification:

```bash
cargo test -p roko-runtime run_ledger
cargo check -p roko-runtime
```

Acceptance:

- Ledger can produce a report without event replay.

## R4: Switch Workflow Report To Ledger

Context files:

- `tmp/subsystem-audits/05-01/19-workflow-result-state-redesign.md`
- `tmp/subsystem-audits/05-01/24-runtime-gate-ledger-plan.md`

Write scope:

- `crates/roko-runtime/src/workflow_engine.rs`
- `crates/roko-runtime/src/run_ledger.rs`
- tests

Mechanical steps:

1. Create a `RunLedger` at workflow start.
2. Record phase transitions and effect outcomes as they occur.
3. Return `ledger.to_report_compat(...)` from `run_with_cancel`.
4. Keep legacy event emission for UI compatibility.
5. Mark event replay report helpers as compatibility only.

Do not:

- Do not remove event logging.
- Do not change pipeline state transitions beyond report source.

Verification:

```bash
cargo test -p roko-runtime workflow_report
cargo check -p roko-runtime
```

Acceptance:

- A test proves report contents remain correct even if event collection is empty or disabled.

## R5: Add GateStatus Type And Compatibility Conversion

Context files:

- `tmp/subsystem-audits/05-01/21-gates-artifact-redesign.md`
- `tmp/subsystem-audits/05-01/24-runtime-gate-ledger-plan.md`

Write scope:

- `crates/roko-core/src/foundation.rs` or new gate type module
- `crates/roko-gate`
- tests

Mechanical steps:

1. Add `GateStatus` enum.
2. Add conversion from existing `passed/skipped/skip_reason` to `GateStatus`.
3. Add tests for passed, failed, skipped, not wired, invalid config if represented.
4. Keep old fields for compatibility in this packet.

Do not:

- Do not migrate all gates yet.
- Do not change runtime decisions yet.

Verification:

```bash
cargo test -p roko-gate gate_status
cargo check -p roko-gate
```

Acceptance:

- Gate status can express skipped/not-wired/invalid separately from failed.

## R6: Add Gate Registry Alias Map

Context files:

- `tmp/subsystem-audits/05-01/21-gates-artifact-redesign.md`
- `tmp/subsystem-audits/05-01/24-runtime-gate-ledger-plan.md`

Write scope:

- `crates/roko-gate/src/registry.rs`
- `crates/roko-gate/src/lib.rs`
- tests

Mechanical steps:

1. Add `GateSpec` and `GateRegistry` with id, aliases, rung, kind, required inputs.
2. Populate current known aliases only: compile, clippy, test, diff, fmt, custom/shell,
   judge.
3. Add tests resolving each alias and rung.

Do not:

- Do not change execution order yet.
- Do not delete duplicate maps yet.

Verification:

```bash
cargo test -p roko-gate gate_registry
cargo check -p roko-gate
```

Acceptance:

- One registry can answer alias and rung for current gates.

## R7: Replace One Duplicate Rung Map With GateRegistry

Context files:

- `tmp/subsystem-audits/05-01/21-gates-artifact-redesign.md`
- `tmp/subsystem-audits/05-01/24-runtime-gate-ledger-plan.md`

Write scope:

- one duplicate map only, preferably `crates/roko-runtime/src/effect_driver.rs`
- tests

Mechanical steps:

1. Replace local rung map with `GateRegistry::resolve`.
2. Preserve behavior for unknown names with an explicit `Unknown`/max-rung fallback
   only if existing tests require it.
3. Add a test that runtime and gate registry agree on rungs.

Do not:

- Do not migrate all CLI maps in this packet.
- Do not change gate execution behavior.

Verification:

```bash
cargo test -p roko-runtime gate_rung
cargo check -p roko-runtime
```

Acceptance:

- One duplicate rung map is gone and behavior is covered by test.

## R8: Add ArtifactOutcome Adapter For GenerationOutcome

Context files:

- `tmp/subsystem-audits/05-01/21-gates-artifact-redesign.md`
- `tmp/subsystem-audits/05-01/24-runtime-gate-ledger-plan.md`

Write scope:

- `crates/roko-cli/src/prd.rs` or a nearby artifact module
- tests

Mechanical steps:

1. Add `ArtifactOutcome` enum if not already present from R3/R5.
2. Add conversion from current `GenerationOutcome` to `ArtifactOutcome`.
3. Add test: `process_success = true` and `artifact_valid = false` converts to
   `ArtifactOutcome::Invalid`.

Do not:

- Do not rewrite PRD generation.
- Do not change CLI output yet.

Verification:

```bash
cargo test -p roko-cli artifact_outcome
cargo check -p roko-cli
```

Acceptance:

- Invalid artifact cannot be represented as pure success in the new adapter.

## R9: Add Typed Command Event DTOs

Context files:

- `tmp/subsystem-audits/05-01/16-terminal-demo-adhoc.md`
- `tmp/subsystem-audits/05-01/24-runtime-gate-ledger-plan.md`

Write scope:

- `crates/roko-serve/src/terminal.rs` or new `command_events.rs`
- tests

Mechanical steps:

1. Add serializable command event DTOs: started, output, exited, spawn failed,
   cancelled.
2. Add tests for JSON serialization.
3. Do not wire demo or terminal sessions yet.

Do not:

- Do not change PTY behavior yet.
- Do not add prompt scraping.

Verification:

```bash
cargo test -p roko-serve command_event
cargo check -p roko-serve
```

Acceptance:

- Typed command events exist and can be serialized for future demo migration.
