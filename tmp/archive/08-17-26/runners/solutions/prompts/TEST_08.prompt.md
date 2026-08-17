# TEST_08: ACP protocol conformance tests

## Tracker

- Issue tracker row: [`ISSUE-TRACKER.md#test-08`](../ISSUE-TRACKER.md#test-08)
- Source: `tmp/solutions/roko/tasks/15-TESTING-VERIFICATION.md` — Task 15.8
- Priority: **P1**
- Effort: 5 hours
- Depends on: `TEST_01` (source 15.1)

When this batch lands, the commit message MUST contain the trailer:

```
tracker: TEST_08 done
```

`bin/sync-tracker.py --apply` flips the corresponding `[ ]` to `[x]`.

## Problem

Existing test infrastructure in `/Users/will/dev/nunchi/roko/roko/crates/roko-acp/tests/protocol_conformance.rs` provides a `TestHarness` with `TestClient` using `tokio::io::DuplexStream` for in-process JSON-RPC communication. This pattern should be reused.

Key types:
- `AcpSession` at `/Users/will/dev/nunchi/roko/roko/crates/roko-acp/src/session.rs` (line 237)
- Error codes: `SESSION_NOT_FOUND`, `METHOD_NOT_FOUND`, `PARSE_ERROR` from `roko_acp::types`

The existing conformance file covers basic JSON-RPC; missing: session lifecycle, mode switching, config updates, slash commands, conversation history, cancellation.

## Exact Changes

1. Test session create -> load -> list -> cancel lifecycle via JSON-RPC
2. Test mode switching: code/plan/research modes produce different system prompts
3. Test config updates: model, effort, temperament, gates, workflow -- all modifiable mid-session
4. Test slash command handling: `/model`, `/effort`, `/status` produce local responses (no agent dispatch)
5. Test conversation history: multi-turn history accumulates correctly
6. Test cancellation: start a pipeline run, cancel mid-execution, verify cooperative shutdown
7. Test error codes: `SESSION_NOT_FOUND` for invalid session IDs, `METHOD_NOT_FOUND` for unknown methods, `PARSE_ERROR` for malformed JSON
8. Test notification delivery: server sends notifications for gate progress, phase transitions
9. Test protocol version negotiation: client sends protocolVersion, server responds
10. Test empty/missing fields: omit optional fields, verify defaults are applied

## Write Scope

_None — this is a documentation/verification-only batch._

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

- [ ] 10+ new tests, all passing
- [ ] Every JSON-RPC error code exercised
- [ ] Session lifecycle fully covered (create, use, resume, cancel, cleanup)

## Verify Recipe

```bash
# Spot-check with ripgrep / git on the touched files.
# Do NOT run cargo — the merge-back pipeline does that.
git diff --stat
```

Then check the commit-message trailer:

```bash
git log -1 --format=%B | rg "^tracker: TEST_08 done"
```

## Acceptance Criteria

- All Verify checkboxes pass on inspection.
- 10+ new tests, all passing
- Every JSON-RPC error code exercised
- Session lifecycle fully covered (create, use, resume, cancel, cleanup)
- No files outside the Write Scope are modified.
- Commit message contains `tracker: TEST_08 done` trailer.
- Pre-commit pipeline (fmt + clippy + test) passes when run by the merge-back stage.

## Do NOT

- Compile or run tests inside the batch (`cargo check/test/clippy/build`).
- Touch files outside the Write Scope above.
- Bundle this batch with another tracker row, even if both touch the same file.
- Add `unwrap()`, `panic!()`, or `unreachable!()` to changed code.
- Add a second dispatch / prompt-assembly / chat-state path. See
  `context-pack/00-RULES.md` §"Universal anti-patterns".
- Refactor neighbours opportunistically. File a separate tracker row.
