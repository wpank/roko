# STAB_20: Wire `roko chat` and dispatch_direct through PromptAssemblyService

## Tracker

- Issue tracker row: [`ISSUE-TRACKER.md#stab-20`](../ISSUE-TRACKER.md#stab-20)
- Source: `tmp/solutions/roko/tasks/01-STABILITY-AND-FIXES.md` — Task 1.20
- Priority: **P1**
- Effort: 3 hours
- Depends on: **none**

When this batch lands, the commit message MUST contain the trailer:

```
tracker: STAB_20 done
```

`bin/sync-tracker.py --apply` flips the corresponding `[ ]` to `[x]`.

## Problem

`roko chat` sends bare prompts to the agent with zero system prompt. No role identity,
no conventions, no knowledge injection. The agent has no context about the project it is
working in.

## Exact Changes

1. In `chat_session.rs`, before sending to agent:
   - Create a `PromptAssemblyService` with lightweight config (skip heavy PRD context)
   - Call `assemble(role="assistant", prompt=user_input)`
   - Pass the assembled system prompt via `--append-system-prompt` to Claude CLI
2. Use a lightweight assembly profile:
   - Include: identity, role, project name, crate structure, conventions
   - Exclude: PRD context, research context, full knowledge dump
   - Budget: 2K tokens for system prompt (keep chat snappy)
3. Cache the assembled system prompt across turns (it doesn't change per-turn).
4. In `dispatch_direct.rs`: apply same treatment if this path is still reachable.

## Design Guidance

The system prompt for chat should be cached and reused across turns since it doesn't change.
Only regenerate it when the model changes (e.g., user runs `/model <new-model>`). This keeps
chat latency low.

## Write Scope

- `crates/roko-cli/src/chat_session.rs`
- `crates/roko-cli/src/dispatch_direct.rs`

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

- [ ] `roko chat` -- type "what project am I working on?" -- agent knows the project name
- [ ] System prompt is under 2K tokens
- [ ] System prompt is cached across turns

## Verify Recipe

```bash
# Spot-check with ripgrep / git on the touched files.
# Do NOT run cargo — the merge-back pipeline does that.
git diff --stat
```

Then check the commit-message trailer:

```bash
git log -1 --format=%B | rg "^tracker: STAB_20 done"
```

## Acceptance Criteria

- All Verify checkboxes pass on inspection.
- `roko chat` -- type "what project am I working on?" -- agent knows the project name
- System prompt is under 2K tokens
- System prompt is cached across turns
- No files outside the Write Scope are modified.
- Commit message contains `tracker: STAB_20 done` trailer.
- Pre-commit pipeline (fmt + clippy + test) passes when run by the merge-back stage.

## Do NOT

- Compile or run tests inside the batch (`cargo check/test/clippy/build`).
- Touch files outside the Write Scope above.
- Bundle this batch with another tracker row, even if both touch the same file.
- Add `unwrap()`, `panic!()`, or `unreachable!()` to changed code.
- Add a second dispatch / prompt-assembly / chat-state path. See
  `context-pack/00-RULES.md` §"Universal anti-patterns".
- Refactor neighbours opportunistically. File a separate tracker row.
