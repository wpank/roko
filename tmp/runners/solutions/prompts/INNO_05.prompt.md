# INNO_05: Implement progressive disclosure context levels

## Tracker

- Issue tracker row: [`ISSUE-TRACKER.md#inno-05`](../ISSUE-TRACKER.md#inno-05)
- Source: `tmp/solutions/roko/tasks/11-INNOVATIONS.md` — Task 11.5
- Priority: **P1**
- Effort: 8 hours
- Depends on: **none**

When this batch lands, the commit message MUST contain the trailer:

```
tracker: INNO_05 done
```

`bin/sync-tracker.py --apply` flips the corresponding `[ ]` to `[x]`.

## Problem

SystemPromptBuilder assembles 9 layers but treats every model the same.
BenchLM.ai's 2026 comparison: effective context can fall 99% below advertised
maximum on complex tasks. Anthropic's context engineering guide (2026):
"context engineering is the discipline of building dynamic systems that provide
the right information at the right time."

JetBrains Research: observation masking (showing agents only relevant
observations while preserving action history) is the single most effective
strategy for software engineering agents.

## Exact Changes

1. Define `DisclosureLevel` enum: `Essential`, `Standard`, `Extended`, `Full`.
2. Tag each SystemPromptBuilder section with a disclosure level:
   - Essential: task description, tool instructions, critical constraints
   - Standard: + role context, code context, recent history
   - Extended: + knowledge injection, playbooks, full file contents
   - Full: + anti-patterns, experimental sections, verbose examples
3. Implement `select_disclosure_level(model_context_window: usize,
   content_tokens: usize) -> DisclosureLevel`:
   - If content fits in 30% of window: Full
   - If content fits in 50% of window: Extended
   - If content fits in 70% of window: Standard
   - Otherwise: Essential (with aggressive trimming)
4. Wire into the prompt assembly path: before assembling, compute total content
   tokens, select level, filter sections by level.
5. Log the selected disclosure level in verbose output.

## Write Scope

- `crates/roko-compose/src/prompt_assembly_service.rs`

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

- [ ] Dispatching to a model with 8K context window produces a prompt at Essential or Standard level
- [ ] Dispatching to Claude Opus (200K) produces a Full-level prompt
- [ ] The section-level filtering is visible in verbose output
- [ ] No prompt exceeds 70% of the model's context window

## Verify Recipe

```bash
# Spot-check with ripgrep / git on the touched files.
# Do NOT run cargo — the merge-back pipeline does that.
git diff --stat
```

Then check the commit-message trailer:

```bash
git log -1 --format=%B | rg "^tracker: INNO_05 done"
```

## Acceptance Criteria

- All Verify checkboxes pass on inspection.
- Dispatching to a model with 8K context window produces a prompt at Essential or Standard level
- Dispatching to Claude Opus (200K) produces a Full-level prompt
- The section-level filtering is visible in verbose output
- No prompt exceeds 70% of the model's context window
- No files outside the Write Scope are modified.
- Commit message contains `tracker: INNO_05 done` trailer.
- Pre-commit pipeline (fmt + clippy + test) passes when run by the merge-back stage.

## Do NOT

- Compile or run tests inside the batch (`cargo check/test/clippy/build`).
- Touch files outside the Write Scope above.
- Bundle this batch with another tracker row, even if both touch the same file.
- Add `unwrap()`, `panic!()`, or `unreachable!()` to changed code.
- Add a second dispatch / prompt-assembly / chat-state path. See
  `context-pack/00-RULES.md` §"Universal anti-patterns".
- Refactor neighbours opportunistically. File a separate tracker row.
