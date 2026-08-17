# TEST_10: Compose and prompt assembly tests

## Tracker

- Issue tracker row: [`ISSUE-TRACKER.md#test-10`](../ISSUE-TRACKER.md#test-10)
- Source: `tmp/solutions/roko/tasks/15-TESTING-VERIFICATION.md` — Task 15.10
- Priority: **P1**
- Effort: 3 hours
- Depends on: `TEST_01` (source 15.1)

When this batch lands, the commit message MUST contain the trailer:

```
tracker: TEST_10 done
```

`bin/sync-tracker.py --apply` flips the corresponding `[ ]` to `[x]`.

## Problem

Key types:
- `SystemPromptBuilder` at `/Users/will/dev/nunchi/roko/roko/crates/roko-compose/src/system_prompt_builder.rs` (line 62) -- 9-layer prompt assembly
- `RoleSystemPromptSpec` at `/Users/will/dev/nunchi/roko/roko/crates/roko-compose/src/role_prompts.rs` (line 228)
- `PromptAssemblyService` at `/Users/will/dev/nunchi/roko/roko/crates/roko-compose/src/prompt_assembly_service.rs` (line 47)

Only 2 existing integration test files (`cache_stability.rs`, `system_prompt_snapshot.rs`). Missing: multi-layer assembly, knowledge injection, episode injection, playbook injection, token budget enforcement, role-specific content.

## Exact Changes

1. Test `SystemPromptBuilder` produces a prompt with all 9 layers present (verify layer markers or section headers)
2. Test `PromptAssemblyService::assemble()` with mock knowledge entries -- verify knowledge appears in assembled prompt
3. Test episode injection: provide mock episodes with prior failures, verify failure context appears
4. Test playbook injection: provide matching playbook, verify guidance appears
5. Test tool instructions: configure tool profiles, verify instructions in output
6. Test `SectionEffectivenessRegistry` weighting: high-lift sections get more token budget
7. Test token budget enforcement: set budget to 1000 tokens, verify assembled prompt is within budget
8. Test template rendering for each role: implementer, reviewer, strategist, researcher, tester -- verify role-specific content differs

## Write Scope

- `crates/roko-compose/Cargo.toml`

## Read-Only Context

The following files contain context that informs this change but **must
not** be modified by this batch. If you find yourself wanting to edit
them, file a follow-up tracker row instead.

- `tmp/runners/solutions/context-pack/00-RULES.md`
- `tmp/runners/solutions/context-pack/02-FILE-INVENTORY.md`
- `tmp/solutions/roko/tasks/15-TESTING-VERIFICATION.md` (the full task list this came from)

## Verification Checklist

These criteria come straight from the source task. Tick each one before
opening the merge:

- [ ] 8+ new tests, all passing
- [ ] All 9 prompt layers verified
- [ ] Token budget enforcement tested

## Verify Recipe

```bash
# Spot-check with ripgrep / git on the touched files.
# Do NOT run cargo — the merge-back pipeline does that.
git diff --stat
```

Then check the commit-message trailer:

```bash
git log -1 --format=%B | rg "^tracker: TEST_10 done"
```

## Acceptance Criteria

- All Verify checkboxes pass on inspection.
- 8+ new tests, all passing
- All 9 prompt layers verified
- Token budget enforcement tested
- No files outside the Write Scope are modified.
- Commit message contains `tracker: TEST_10 done` trailer.
- Pre-commit pipeline (fmt + clippy + test) passes when run by the merge-back stage.

## Do NOT

- Compile or run tests inside the batch (`cargo check/test/clippy/build`).
- Touch files outside the Write Scope above.
- Bundle this batch with another tracker row, even if both touch the same file.
- Add `unwrap()`, `panic!()`, or `unreachable!()` to changed code.
- Add a second dispatch / prompt-assembly / chat-state path. See
  `context-pack/00-RULES.md` §"Universal anti-patterns".
- Refactor neighbours opportunistically. File a separate tracker row.
