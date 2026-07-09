# STAB_29: Replace ACP direct subprocess spawns with provider system

## Tracker

- Issue tracker row: [`ISSUE-TRACKER.md#stab-29`](../ISSUE-TRACKER.md#stab-29)
- Source: `tmp/solutions/roko/tasks/01-STABILITY-AND-FIXES.md` — Task 1.29
- Priority: **P1**
- Effort: 4 hours
- Depends on: **none**

When this batch lands, the commit message MUST contain the trailer:

```
tracker: STAB_29 done
```

`bin/sync-tracker.py --apply` flips the corresponding `[ ]` to `[x]`.

## Problem

Two ACP paths bypass the provider system: `run_claude_cli()` spawns a bare subprocess with
no model flag, no streaming, no system prompt, no feedback. `run_claude_cognitive_task()`
builds its own subprocess.

## Exact Changes

1. Replace `run_claude_cli()` calls with `create_agent_for_model()` via the provider adapter.
2. Replace `run_claude_cognitive_task()` similarly.
3. Replace `run_openai_compat_cognitive_task()` with provider adapter calls.
4. Pass model, system prompt, and feedback service through the provider adapter.
5. Cost tracking and credential management now happen automatically via the adapter.

## Design Guidance

All model calls should go through the provider adapter system. This ensures consistent
credential management, cost tracking, rate limiting, and circuit breaking.

## Write Scope

- `crates/roko-acp/src/runner.rs`

## Read-Only Context

The following files contain context that informs this change but **must
not** be modified by this batch. If you find yourself wanting to edit
them, file a follow-up tracker row instead.

- `tmp/runners/solutions/context-pack/00-RULES.md`
- `tmp/runners/solutions/context-pack/02-FILE-INVENTORY.md`
- `tmp/solutions/roko/tasks/01-STABILITY-AND-FIXES.md` (the full task list this came from)

## Verification Checklist

These criteria come straight from the source task. Tick each one before
opening the merge:

- [ ] ACP model calls appear in `.roko/learn/efficiency.jsonl`
- [ ] Cost tracking shows non-zero values for ACP sessions
- [ ] `grep -rn 'run_claude_cli\|run_claude_cognitive' crates/roko-acp/` returns zero matches

## Verify Recipe

```bash
# Spot-check with ripgrep / git on the touched files.
# Do NOT run cargo — the merge-back pipeline does that.
git diff --stat
```

Then check the commit-message trailer:

```bash
git log -1 --format=%B | rg "^tracker: STAB_29 done"
```

## Acceptance Criteria

- All Verify checkboxes pass on inspection.
- ACP model calls appear in `.roko/learn/efficiency.jsonl`
- Cost tracking shows non-zero values for ACP sessions
- `grep -rn 'run_claude_cli\|run_claude_cognitive' crates/roko-acp/` returns zero matches
- No files outside the Write Scope are modified.
- Commit message contains `tracker: STAB_29 done` trailer.
- Pre-commit pipeline (fmt + clippy + test) passes when run by the merge-back stage.

## Do NOT

- Compile or run tests inside the batch (`cargo check/test/clippy/build`).
- Touch files outside the Write Scope above.
- Bundle this batch with another tracker row, even if both touch the same file.
- Add `unwrap()`, `panic!()`, or `unreachable!()` to changed code.
- Add a second dispatch / prompt-assembly / chat-state path. See
  `context-pack/00-RULES.md` §"Universal anti-patterns".
- Refactor neighbours opportunistically. File a separate tracker row.
