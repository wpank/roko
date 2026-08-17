# INNO_03: Wire MemoryLayer into SystemPromptBuilder

## Tracker

- Issue tracker row: [`ISSUE-TRACKER.md#inno-03`](../ISSUE-TRACKER.md#inno-03)
- Source: `tmp/solutions/roko/tasks/11-INNOVATIONS.md` — Task 11.3
- Priority: **P0**
- Effort: 4 hours
- Depends on: `INNO_02` (source 11.2)

When this batch lands, the commit message MUST contain the trailer:

```
tracker: INNO_03 done
```

`bin/sync-tracker.py --apply` flips the corresponding `[ ]` to `[x]`.

## Problem

SystemPromptBuilder at `crates/roko-compose/src/system_prompt_builder.rs`
assembles 9 layers. Layers 6 (Techniques) and 7 (Anti-patterns) are the natural
injection points for memory-derived content. PromptAssemblyService at
`crates/roko-compose/src/prompt_assembly_service.rs` orchestrates the assembly.

The dispatch path (search `dispatch` in `crates/roko-cli/src/dispatch/mod.rs`
and `crates/roko-cli/src/runner/event_loop.rs`) is where the system prompt is
built before agent invocation.

## Exact Changes

1. In the dispatch path, construct or receive a `MemoryLayer` instance.
2. Call `memory_layer.query_for_task(&task_context)` to get `MemoryInjection`.
3. Format playbooks as layer 6 content: brief summaries with confidence scores.
   Example: "Playbook: {name} (confidence: {score})\n{steps_summary}".
4. Format anti-patterns as layer 7 content: "AVOID: {pattern} (seen {count}
   times, last failure: {date})".
5. Format relevant episodes as layer 4 supplemental content: "Prior attempt on
   similar task {task_id}: {outcome}. Key insight: {insight}."
6. Pass formatted sections to `PromptAssemblyService` as additional layer
   content. Respect existing token budget: if VCG auction is active, memory
   sections participate as bidders; otherwise, append within budget.
7. Add `--verbose` output showing memory injection contents for debugging.

## Write Scope

- `crates/roko-compose/src/prompt_assembly_service.rs`
- `crates/roko-cli/src/dispatch/mod.rs`

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

- [ ] Run `roko plan run` on a plan where prior runs recorded episodes
- [ ] Inspect the system prompt (via `--verbose` or episode log): layers 6/7 contain memory-derived content
- [ ] A task that previously failed sees the failure pattern in its anti-patterns section
- [ ] Memory injection does not exceed the overall prompt token budget

## Verify Recipe

```bash
# Spot-check with ripgrep / git on the touched files.
# Do NOT run cargo — the merge-back pipeline does that.
git diff --stat
```

Then check the commit-message trailer:

```bash
git log -1 --format=%B | rg "^tracker: INNO_03 done"
```

## Acceptance Criteria

- All Verify checkboxes pass on inspection.
- Run `roko plan run` on a plan where prior runs recorded episodes
- Inspect the system prompt (via `--verbose` or episode log): layers 6/7 contain memory-derived content
- A task that previously failed sees the failure pattern in its anti-patterns section
- Memory injection does not exceed the overall prompt token budget
- No files outside the Write Scope are modified.
- Commit message contains `tracker: INNO_03 done` trailer.
- Pre-commit pipeline (fmt + clippy + test) passes when run by the merge-back stage.

## Do NOT

- Compile or run tests inside the batch (`cargo check/test/clippy/build`).
- Touch files outside the Write Scope above.
- Bundle this batch with another tracker row, even if both touch the same file.
- Add `unwrap()`, `panic!()`, or `unreachable!()` to changed code.
- Add a second dispatch / prompt-assembly / chat-state path. See
  `context-pack/00-RULES.md` §"Universal anti-patterns".
- Refactor neighbours opportunistically. File a separate tracker row.
