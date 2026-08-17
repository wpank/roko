# GATE_25: Consolidate GatePipeline and ComposedGatePipeline

## Tracker

- Issue tracker row: [`ISSUE-TRACKER.md#gate-25`](../ISSUE-TRACKER.md#gate-25)
- Source: `tmp/solutions/roko/tasks/04-GATE-PIPELINE.md` — Task 4.25
- Priority: **P2**
- Effort: 3 hours
- Depends on: **none**

When this batch lands, the commit message MUST contain the trailer:

```
tracker: GATE_25 done
```

`bin/sync-tracker.py --apply` flips the corresponding `[ ]` to `[x]`.

## Problem

`GatePipeline` and `ComposedGatePipeline` in `crates/roko-gate/src/gate_pipeline.rs` are parallel implementations (AP-DUP). `ComposedGatePipeline::Sequential` mode re-implements the loop from `GatePipeline` rather than delegating. There is dead code: `let _ = pipeline;`.

## Exact Changes

1. Make `ComposedGatePipeline::Sequential` delegate to `GatePipeline::verify()`:
   ```rust
   GateComposition::Sequential => {
       let pipeline = GatePipeline::new(&self.name);
       // Actually use the pipeline
       for gate in &self.gates {
           pipeline.push(gate.clone());
       }
       return pipeline.verify(signal, ctx).await;
   }
   ```
2. Or deprecate `GatePipeline` in favor of `ComposedGatePipeline` with a default Sequential mode.
3. Remove the dead `let _ = pipeline;` code.
4. Add `#[deprecated]` annotation if keeping both types.

## Write Scope

- `crates/roko-gate/src/gate_pipeline.rs`

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

- [ ] No dead code warnings from `GatePipeline` usage
- [ ] Sequential mode behavior is identical (short-circuit semantics preserved)

## Verify Recipe

```bash
# Spot-check with ripgrep / git on the touched files.
# Do NOT run cargo — the merge-back pipeline does that.
git diff --stat
```

Then check the commit-message trailer:

```bash
git log -1 --format=%B | rg "^tracker: GATE_25 done"
```

## Acceptance Criteria

- All Verify checkboxes pass on inspection.
- No dead code warnings from `GatePipeline` usage
- Sequential mode behavior is identical (short-circuit semantics preserved)
- No files outside the Write Scope are modified.
- Commit message contains `tracker: GATE_25 done` trailer.
- Pre-commit pipeline (fmt + clippy + test) passes when run by the merge-back stage.

## Do NOT

- Compile or run tests inside the batch (`cargo check/test/clippy/build`).
- Touch files outside the Write Scope above.
- Bundle this batch with another tracker row, even if both touch the same file.
- Add `unwrap()`, `panic!()`, or `unreachable!()` to changed code.
- Add a second dispatch / prompt-assembly / chat-state path. See
  `context-pack/00-RULES.md` §"Universal anti-patterns".
- Refactor neighbours opportunistically. File a separate tracker row.
