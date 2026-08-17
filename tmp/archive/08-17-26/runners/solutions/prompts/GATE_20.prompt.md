# GATE_20: Wire custom gates into GateService

## Tracker

- Issue tracker row: [`ISSUE-TRACKER.md#gate-20`](../ISSUE-TRACKER.md#gate-20)
- Source: `tmp/solutions/roko/tasks/04-GATE-PIPELINE.md` — Task 4.20
- Priority: **P2**
- Effort: 3 hours
- Depends on: `GATE_19` (source 4.19)

When this batch lands, the commit message MUST contain the trailer:

```
tracker: GATE_20 done
```

`bin/sync-tracker.py --apply` flips the corresponding `[ ]` to `[x]`.

## Problem

GateService currently resolves gate names via `gate_for_name()` which returns concrete gate implementations for known names (compile, clippy, test, diff, fmt) and returns `None` for unknown names. Custom gates from config should be resolvable by name.

## Exact Changes

1. Add a `custom_gates: HashMap<String, CustomGateSpec>` field to `GateService`.
2. Add builder method:
   ```rust
   pub fn with_custom_gates(mut self, specs: Vec<CustomGateSpec>) -> Self {
       for spec in specs {
           self.custom_gates.insert(spec.name.clone(), spec);
       }
       self
   }
   ```
3. In `run_gates()`, when processing gate names, check custom_gates before the default `gate_for_name()`:
   ```rust
   if let Some(custom) = self.custom_gates.get(&gate_name) {
       let gate = ShellGate::new(&custom.program, custom.args.clone())
           .with_name(&custom.name)
           .with_timeout_ms(custom.timeout_ms);
       // Run gate...
   }
   ```
4. Override `rung_for_name()` to check custom gates:
   ```rust
   fn rung_for_name_with_custom(&self, name: &str) -> Option<u8> {
       if let Some(custom) = self.custom_gates.get(name) {
           Some(custom.rung)
       } else {
           Self::rung_for_name(name)
       }
   }
   ```
5. In the ServiceFactory or WorkflowEngine config builder, read `[[gate.custom]]` sections from `roko.toml` and pass them as `CustomGateSpec` to GateService.

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

- [ ] Custom gate "my-check" with program "make" and args ["lint"] executes via GateService
- [ ] Custom gates are ordered by their configured rung value
- [ ] Unknown custom gate names produce skipped verdicts, not panics

## Verify Recipe

```bash
# Spot-check with ripgrep / git on the touched files.
# Do NOT run cargo — the merge-back pipeline does that.
git diff --stat
```

Then check the commit-message trailer:

```bash
git log -1 --format=%B | rg "^tracker: GATE_20 done"
```

## Acceptance Criteria

- All Verify checkboxes pass on inspection.
- Custom gate "my-check" with program "make" and args ["lint"] executes via GateService
- Custom gates are ordered by their configured rung value
- Unknown custom gate names produce skipped verdicts, not panics
- No files outside the Write Scope are modified.
- Commit message contains `tracker: GATE_20 done` trailer.
- Pre-commit pipeline (fmt + clippy + test) passes when run by the merge-back stage.

## Do NOT

- Compile or run tests inside the batch (`cargo check/test/clippy/build`).
- Touch files outside the Write Scope above.
- Bundle this batch with another tracker row, even if both touch the same file.
- Add `unwrap()`, `panic!()`, or `unreachable!()` to changed code.
- Add a second dispatch / prompt-assembly / chat-state path. See
  `context-pack/00-RULES.md` §"Universal anti-patterns".
- Refactor neighbours opportunistically. File a separate tracker row.
