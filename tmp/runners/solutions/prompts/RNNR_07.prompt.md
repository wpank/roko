# RNNR_07: Implement no-build context injection

## Tracker

- Issue tracker row: [`ISSUE-TRACKER.md#rnnr-07`](../ISSUE-TRACKER.md#rnnr-07)
- Source: `tmp/solutions/roko/tasks/14-RUNNER-PATTERNS.md` — Task 14.7
- Priority: **??**
- Effort: ?
- Depends on: `RNNR_06` (source 14.6)

When this batch lands, the commit message MUST contain the trailer:

```
tracker: RNNR_07 done
```

`bin/sync-tracker.py --apply` flips the corresponding `[ ]` to `[x]`.

## Problem

: When wave gating is active, inject a "do not compile" instruction
into the system prompt. Mega-parity runner proved this reduces task time from
15-40 min to 1-5 min with ~95% compliance (99% when placed in system prompt).

## Exact Changes

1. Add `BuildPolicy` enum to the prompt assembly module:
   ```rust
   #[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
   pub enum BuildPolicy {
       #[default]
       Allowed,
       Prohibited,
   }
   ```
2. Add `build_policy: BuildPolicy` to `PromptSpec` (or the assembly input struct)
3. When `BuildPolicy::Prohibited`, inject as a high-priority system prompt
   section (layer 1, before task description):
   ```
   IMPORTANT: Do NOT run `cargo build`, `cargo check`, `cargo test`, `cargo clippy`,
   or any other compilation command. The runner will verify your changes at the wave
   gate. Focus only on writing correct code.
   ```
4. Place in layer 1 (not a context file) per lesson: system prompt placement
   achieves 99% compliance vs 95% for context files
5. When `WaveGateMode::PerWave` or `Deferred`, automatically set `BuildPolicy::Prohibited`

## Write Scope

_None — this is a documentation/verification-only batch._

## Read-Only Context

The following files contain context that informs this change but **must
not** be modified by this batch. If you find yourself wanting to edit
them, file a follow-up tracker row instead.

- `tmp/runners/solutions/context-pack/00-RULES.md`
- `tmp/runners/solutions/context-pack/02-FILE-INVENTORY.md`
- `tmp/solutions/roko/tasks/14-RUNNER-PATTERNS.md` (the full task list this came from)

## Verification Checklist

These criteria come straight from the source task. Tick each one before
opening the merge:

- [ ] Agents dispatched under wave gating receive the no-build instruction
- [ ] Instruction appears early in the system prompt (layer 1)
- [ ] Agents dispatched without wave gating do NOT receive the instruction
- [ ] Per-task override allows specific tasks to build when needed

## Verify Recipe

```bash
# Spot-check with ripgrep / git on the touched files.
# Do NOT run cargo — the merge-back pipeline does that.
git diff --stat
```

Then check the commit-message trailer:

```bash
git log -1 --format=%B | rg "^tracker: RNNR_07 done"
```

## Acceptance Criteria

- All Verify checkboxes pass on inspection.
- Agents dispatched under wave gating receive the no-build instruction
- Instruction appears early in the system prompt (layer 1)
- Agents dispatched without wave gating do NOT receive the instruction
- Per-task override allows specific tasks to build when needed
- No files outside the Write Scope are modified.
- Commit message contains `tracker: RNNR_07 done` trailer.
- Pre-commit pipeline (fmt + clippy + test) passes when run by the merge-back stage.

## Do NOT

- Compile or run tests inside the batch (`cargo check/test/clippy/build`).
- Touch files outside the Write Scope above.
- Bundle this batch with another tracker row, even if both touch the same file.
- Add `unwrap()`, `panic!()`, or `unreachable!()` to changed code.
- Add a second dispatch / prompt-assembly / chat-state path. See
  `context-pack/00-RULES.md` §"Universal anti-patterns".
- Refactor neighbours opportunistically. File a separate tracker row.
