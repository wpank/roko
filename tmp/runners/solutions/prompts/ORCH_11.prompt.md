# ORCH_11: Implement KnowledgeRouter for WorkflowEngine

## Tracker

- Issue tracker row: [`ISSUE-TRACKER.md#orch-11`](../ISSUE-TRACKER.md#orch-11)
- Source: `tmp/solutions/roko/tasks/02-ORCHESTRATION.md` — Task 2.11
- Priority: **P1**
- Effort: 5 hours
- Depends on: `ORCH_10` (source 2.10)

When this batch lands, the commit message MUST contain the trailer:

```
tracker: ORCH_11 done
```

`bin/sync-tracker.py --apply` flips the corresponding `[ ]` to `[x]`.

## Problem

orchestrate.rs has `build_knowledge_routing_advice()` which queries the neuro (knowledge) store for context relevant to the current task. The `KnowledgeStore` exists at `crates/roko-neuro/` and is fully functional. The `ContextAssembler` and `TierProgression` types exist there too.

This task implements the `KnowledgeRouter` trait using the existing `KnowledgeStore`, then wires it into EffectDriver so that dispatched agents receive knowledge context.

## Exact Changes

1. Create a `NeuroKnowledgeRouter` struct in `crates/roko-neuro/` that wraps `KnowledgeStore`.
2. Implement the `KnowledgeRouter` trait from `roko-core::foundation`:
   - `route()` queries the knowledge store for entries matching the task description
   - Returns relevant knowledge entries as formatted strings
3. In the EffectDriver's `spawn_agent()` method, if `knowledge_router` is `Some`, query it before prompt assembly and inject results into the `PromptSpec::gate_feedback` or a new context section.
4. Wire the router construction in CLI entry points where `KnowledgeStore` is already loaded.

## Design Guidance

The knowledge router should be stateless (queries only, no writes). Writes happen via the episode recording path. Keep the query lightweight -- limit to 5 results, with total token budget of 2000 tokens for knowledge context.

## Write Scope

- `crates/roko-neuro/src/lib.rs`
- `crates/roko-runtime/src/effect_driver.rs`

## Read-Only Context

The following files contain context that informs this change but **must
not** be modified by this batch. If you find yourself wanting to edit
them, file a follow-up tracker row instead.

- `tmp/runners/solutions/context-pack/00-RULES.md`
- `tmp/runners/solutions/context-pack/02-FILE-INVENTORY.md`
- `tmp/solutions/roko/tasks/02-ORCHESTRATION.md` (the full task list this came from)

## Verification Checklist

These criteria come straight from the source task. Tick each one before
opening the merge:

- [ ] `NeuroKnowledgeRouter` implements `KnowledgeRouter` trait
- [ ] EffectDriver queries knowledge router before agent dispatch (when available)
- [ ] Knowledge context appears in the prompt sent to the agent
- [ ] Graceful degradation: when knowledge_router is None, behavior is unchanged

## Verify Recipe

```bash
# Spot-check with ripgrep / git on the touched files.
# Do NOT run cargo — the merge-back pipeline does that.
git diff --stat
```

Then check the commit-message trailer:

```bash
git log -1 --format=%B | rg "^tracker: ORCH_11 done"
```

## Acceptance Criteria

- All Verify checkboxes pass on inspection.
- `NeuroKnowledgeRouter` implements `KnowledgeRouter` trait
- EffectDriver queries knowledge router before agent dispatch (when available)
- Knowledge context appears in the prompt sent to the agent
- Graceful degradation: when knowledge_router is None, behavior is unchanged
- No files outside the Write Scope are modified.
- Commit message contains `tracker: ORCH_11 done` trailer.
- Pre-commit pipeline (fmt + clippy + test) passes when run by the merge-back stage.

## Do NOT

- Compile or run tests inside the batch (`cargo check/test/clippy/build`).
- Touch files outside the Write Scope above.
- Bundle this batch with another tracker row, even if both touch the same file.
- Add `unwrap()`, `panic!()`, or `unreachable!()` to changed code.
- Add a second dispatch / prompt-assembly / chat-state path. See
  `context-pack/00-RULES.md` §"Universal anti-patterns".
- Refactor neighbours opportunistically. File a separate tracker row.
