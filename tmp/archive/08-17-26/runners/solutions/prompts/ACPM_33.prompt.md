# ACPM_33: Publish Roko Agent Card

## Tracker

- Issue tracker row: [`ISSUE-TRACKER.md#acpm-33`](../ISSUE-TRACKER.md#acpm-33)
- Source: `tmp/solutions/roko/tasks/09-ACP-MCP.md` — Task 9.33
- Priority: **P2**
- Effort: 3 hours
- Depends on: `ACPM_32` (source 9.32)

When this batch lands, the commit message MUST contain the trailer:

```
tracker: ACPM_33 done
```

`bin/sync-tracker.py --apply` flips the corresponding `[ ]` to `[x]`.

## Problem

The A2A spec requires agents to publish their capabilities at `/.well-known/agent.json`. `roko-serve` already has ~85 routes; adding one more for agent card is straightforward.

## Exact Changes

1. Implement `fn build_agent_card(config: &RokoConfig) -> AgentCard` in `agent_card.rs`:
   - Skills: "code_implementation", "code_review", "research_and_analysis", "plan_generation"
   - Default input/output modes: `["text/plain", "application/json"]`
   - Authentication: `[{ scheme: "bearer" }]`
   - Version from `env!("CARGO_PKG_VERSION")`
2. Add `GET /.well-known/agent.json` route to `roko-serve`'s router.
3. The route handler constructs the card from the server's config and returns it as JSON.

## Write Scope

- `crates/roko-serve/src/routes/mod.rs`

## Read-Only Context

The following files contain context that informs this change but **must
not** be modified by this batch. If you find yourself wanting to edit
them, file a follow-up tracker row instead.

- `tmp/runners/solutions/context-pack/00-RULES.md`
- `tmp/runners/solutions/context-pack/02-FILE-INVENTORY.md`
- `tmp/solutions/roko/tasks/09-ACP-MCP.md` (the full task list this came from)

## Verification Checklist

These criteria come straight from the source task. Tick each one before
opening the merge:

- [ ] `GET /.well-known/agent.json` returns valid Agent Card JSON
- [ ] Card includes 4 skills
- [ ] Version matches crate version

## Verify Recipe

```bash
# Spot-check with ripgrep / git on the touched files.
# Do NOT run cargo — the merge-back pipeline does that.
git diff --stat
```

Then check the commit-message trailer:

```bash
git log -1 --format=%B | rg "^tracker: ACPM_33 done"
```

## Acceptance Criteria

- All Verify checkboxes pass on inspection.
- `GET /.well-known/agent.json` returns valid Agent Card JSON
- Card includes 4 skills
- Version matches crate version
- No files outside the Write Scope are modified.
- Commit message contains `tracker: ACPM_33 done` trailer.
- Pre-commit pipeline (fmt + clippy + test) passes when run by the merge-back stage.

## Do NOT

- Compile or run tests inside the batch (`cargo check/test/clippy/build`).
- Touch files outside the Write Scope above.
- Bundle this batch with another tracker row, even if both touch the same file.
- Add `unwrap()`, `panic!()`, or `unreachable!()` to changed code.
- Add a second dispatch / prompt-assembly / chat-state path. See
  `context-pack/00-RULES.md` §"Universal anti-patterns".
- Refactor neighbours opportunistically. File a separate tracker row.
