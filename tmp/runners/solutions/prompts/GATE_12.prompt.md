# GATE_12: Add temperament to GateService

## Tracker

- Issue tracker row: [`ISSUE-TRACKER.md#gate-12`](../ISSUE-TRACKER.md#gate-12)
- Source: `tmp/solutions/roko/tasks/04-GATE-PIPELINE.md` — Task 4.12
- Priority: **P1**
- Effort: 2 hours
- Depends on: `GATE_01` (source 4.1)

When this batch lands, the commit message MUST contain the trailer:

```
tracker: GATE_12 done
```

`bin/sync-tracker.py --apply` flips the corresponding `[ ]` to `[x]`.

## Problem

`Temperament` at `crates/roko-core/src/temperament.rs:14` has variants: Conservative, Balanced, Aggressive, Exploratory. `AdaptiveThresholds::should_skip_rung_for_temperament()` at `crates/roko-gate/src/adaptive_threshold.rs:602` implements temperament-aware skip logic (Conservative never skips, Aggressive skips at half the streak threshold). But GateService ignores temperament -- it always calls `should_skip_rung()` (the temperament-unaware version).

## Exact Changes

1. Add `temperament: Option<String>` field to `GateConfig` (string to avoid roko-core importing Temperament directly -- or use the `Temperament` type since it is already in roko-core).
2. Actually, since `Temperament` is in roko-core (`crates/roko-core/src/temperament.rs`), use it directly:
   ```rust
   pub temperament: Option<Temperament>,
   ```
3. Add `temperament` field to `GateService`:
   ```rust
   pub struct GateService {
       adaptive: Option<Arc<Mutex<AdaptiveThresholds>>>,
       temperament: Temperament,
   }
   ```
4. Add builder method:
   ```rust
   pub fn with_temperament(mut self, t: Temperament) -> Self {
       self.temperament = t;
       self
   }
   ```
5. Update `should_skip_rung_adaptively()` to use temperament-aware method:
   ```rust
   fn should_skip_rung_adaptively(&self, rung: Option<u8>) -> Result<bool> {
       // ... existing None check ...
       thresholds.should_skip_rung_for_temperament(u32::from(r), self.temperament)
       // ... rest unchanged ...
   }
   ```
6. Alternatively, read temperament from GateConfig at the start of run_gates() and use it throughout.

## Write Scope

- `crates/roko-gate/src/gate_service.rs`
- `crates/roko-core/src/foundation.rs`

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

- [ ] Conservative temperament never skips any rung
- [ ] Aggressive temperament skips at half the normal streak threshold

## Verify Recipe

```bash
# Spot-check with ripgrep / git on the touched files.
# Do NOT run cargo — the merge-back pipeline does that.
git diff --stat
```

Then check the commit-message trailer:

```bash
git log -1 --format=%B | rg "^tracker: GATE_12 done"
```

## Acceptance Criteria

- All Verify checkboxes pass on inspection.
- Conservative temperament never skips any rung
- Aggressive temperament skips at half the normal streak threshold
- No files outside the Write Scope are modified.
- Commit message contains `tracker: GATE_12 done` trailer.
- Pre-commit pipeline (fmt + clippy + test) passes when run by the merge-back stage.

## Do NOT

- Compile or run tests inside the batch (`cargo check/test/clippy/build`).
- Touch files outside the Write Scope above.
- Bundle this batch with another tracker row, even if both touch the same file.
- Add `unwrap()`, `panic!()`, or `unreachable!()` to changed code.
- Add a second dispatch / prompt-assembly / chat-state path. See
  `context-pack/00-RULES.md` §"Universal anti-patterns".
- Refactor neighbours opportunistically. File a separate tracker row.
