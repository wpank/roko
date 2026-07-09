# TEST_06: Agent dispatcher integration tests

## Tracker

- Issue tracker row: [`ISSUE-TRACKER.md#test-06`](../ISSUE-TRACKER.md#test-06)
- Source: `tmp/solutions/roko/tasks/15-TESTING-VERIFICATION.md` — Task 15.6
- Priority: **P1**
- Effort: 4 hours
- Depends on: `TEST_01` (source 15.1)

When this batch lands, the commit message MUST contain the trailer:

```
tracker: TEST_06 done
```

`bin/sync-tracker.py --apply` flips the corresponding `[ ]` to `[x]`.

## Problem

roko-agent has 23 integration test files focused on individual provider parity (openai, codex, cursor, gemini, kimi, ollama, etc.) and safety. Missing: integration tests for the dispatcher layer itself, tool loop, and MCP passthrough.

Key types:
- Dispatcher at `/Users/will/dev/nunchi/roko/roko/crates/roko-agent/src/dispatcher/mod.rs`
- Mock provider test at `/Users/will/dev/nunchi/roko/roko/crates/roko-agent/tests/mock_provider.rs`
- Safety integration at `/Users/will/dev/nunchi/roko/roko/crates/roko-agent/tests/safety_integration.rs`
- Tool loop at `/Users/will/dev/nunchi/roko/roko/crates/roko-agent/tests/tool_loop_integration.rs`

## Exact Changes

1. Test mock provider dispatches correctly: configure mock, send prompt, verify response contains expected text
2. Test provider selection: configure 2 providers, verify dispatch routes to requested provider
3. Test tool loop: mock provider returns a tool call, verify tool is executed and result fed back
4. Test tool loop max iterations: configure `max_iterations=3`, verify loop terminates after 3 rounds
5. Test safety contract enforcement: configure deny-list, attempt denied tool, verify rejection
6. Test MCP config passthrough: provide `mcp_config` path, verify it is passed to the agent process
7. Test stream accumulation: mock provider returns 5 stream chunks, verify accumulator produces complete message
8. Test error handling: mock provider returns error, verify dispatch returns structured error (not panic)

## Write Scope

- `crates/roko-agent/Cargo.toml`

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
- [ ] Mock provider tests do not require API keys
- [ ] Tool loop tested with at least one tool call round-trip

## Verify Recipe

```bash
# Spot-check with ripgrep / git on the touched files.
# Do NOT run cargo — the merge-back pipeline does that.
git diff --stat
```

Then check the commit-message trailer:

```bash
git log -1 --format=%B | rg "^tracker: TEST_06 done"
```

## Acceptance Criteria

- All Verify checkboxes pass on inspection.
- 8+ new tests, all passing
- Mock provider tests do not require API keys
- Tool loop tested with at least one tool call round-trip
- No files outside the Write Scope are modified.
- Commit message contains `tracker: TEST_06 done` trailer.
- Pre-commit pipeline (fmt + clippy + test) passes when run by the merge-back stage.

## Do NOT

- Compile or run tests inside the batch (`cargo check/test/clippy/build`).
- Touch files outside the Write Scope above.
- Bundle this batch with another tracker row, even if both touch the same file.
- Add `unwrap()`, `panic!()`, or `unreachable!()` to changed code.
- Add a second dispatch / prompt-assembly / chat-state path. See
  `context-pack/00-RULES.md` §"Universal anti-patterns".
- Refactor neighbours opportunistically. File a separate tracker row.
