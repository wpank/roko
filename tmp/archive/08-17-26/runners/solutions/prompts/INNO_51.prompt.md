# INNO_51: Add model-heterogeneity enforcement in gate judges

## Tracker

- Issue tracker row: [`ISSUE-TRACKER.md#inno-51`](../ISSUE-TRACKER.md#inno-51)
- Source: `tmp/solutions/roko/tasks/11-INNOVATIONS.md` — Task 11.51
- Priority: **P1**
- Effort: 4 hours
- Depends on: **none**

When this batch lands, the commit message MUST contain the trailer:

```
tracker: INNO_51 done
```

`bin/sync-tracker.py --apply` flips the corresponding `[ ]` to `[x]`.

## Problem

`enrich_rung_config` is called from `crates/roko-cli/src/orchestrate.rs` and
`crates/roko-cli/src/gate_runner.rs`. Oracle gate rungs (4-6) can currently
use the same model family as the task agent.

Research: ICLR 2026 (arxiv 2502.01534) -- evaluation breaks when judge and
generator share a lineage. "Great Models Think Alike" (ICML 2025) -- debate
value collapses to zero when debater models share weights.

## Exact Changes

1. In `enrich_rung_config()` or its caller, accept the agent's model slug.
2. Determine the agent's model family (Claude, GPT, Gemini, etc.) by prefix.
3. For oracle rungs (4-6), select a judge model from a different family.
4. If no alternative model is configured, log a warning and proceed with
   the same family (degraded mode, not a hard failure).
5. Record judge model in the gate verdict for auditability.

## Write Scope

- `crates/roko-gate/src/gate_service.rs`
- `crates/roko-cli/src/gate_runner.rs`

## Read-Only Context

The following files contain context that informs this change but **must
not** be modified by this batch. If you find yourself wanting to edit
them, file a follow-up tracker row instead.

- `tmp/runners/solutions/context-pack/00-RULES.md`
- `tmp/runners/solutions/context-pack/02-FILE-INVENTORY.md`
- `tmp/solutions/roko/tasks/11-INNOVATIONS.md` (the full task list this came from)

## Verification Checklist

These criteria come straight from the source task. Tick each one before
opening the merge:

- [ ] Agent uses Claude Sonnet -> oracle judge uses GPT or Gemini
- [ ] If only Claude models are configured, a warning is logged
- [ ] Judge model is visible in gate verdict logs

## Verify Recipe

```bash
# Spot-check with ripgrep / git on the touched files.
# Do NOT run cargo — the merge-back pipeline does that.
git diff --stat
```

Then check the commit-message trailer:

```bash
git log -1 --format=%B | rg "^tracker: INNO_51 done"
```

## Acceptance Criteria

- All Verify checkboxes pass on inspection.
- Agent uses Claude Sonnet -> oracle judge uses GPT or Gemini
- If only Claude models are configured, a warning is logged
- Judge model is visible in gate verdict logs
- No files outside the Write Scope are modified.
- Commit message contains `tracker: INNO_51 done` trailer.
- Pre-commit pipeline (fmt + clippy + test) passes when run by the merge-back stage.

## Do NOT

- Compile or run tests inside the batch (`cargo check/test/clippy/build`).
- Touch files outside the Write Scope above.
- Bundle this batch with another tracker row, even if both touch the same file.
- Add `unwrap()`, `panic!()`, or `unreachable!()` to changed code.
- Add a second dispatch / prompt-assembly / chat-state path. See
  `context-pack/00-RULES.md` §"Universal anti-patterns".
- Refactor neighbours opportunistically. File a separate tracker row.
