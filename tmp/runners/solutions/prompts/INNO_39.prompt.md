# INNO_39: Implement A2A client for external agent delegation

## Tracker

- Issue tracker row: [`ISSUE-TRACKER.md#inno-39`](../ISSUE-TRACKER.md#inno-39)
- Source: `tmp/solutions/roko/tasks/11-INNOVATIONS.md` — Task 11.39
- Priority: **P3**
- Effort: 12 hours
- Depends on: `INNO_36` (source 11.36)

When this batch lands, the commit message MUST contain the trailer:

```
tracker: INNO_39 done
```

`bin/sync-tracker.py --apply` flips the corresponding `[ ]` to `[x]`.

## Problem

_(no context section in source)_

## Exact Changes

1. Define `A2AClient` struct with `reqwest::Client`.
2. Implement `discover(card_url: &str) -> Result<AgentCard>`.
3. Implement `send_task(card: &AgentCard, skill_id: &str, input: &str)
   -> Result<A2ATask>`.
4. Add `[a2a.agents]` config section to roko.toml.
5. At dispatch time, check if task domain matches an A2A agent's skills.
   If so, delegate via A2A. Fallback to local agent on failure.

## Write Scope

- `crates/roko-agent/src/lib.rs`

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

- [ ] Configure an external agent in roko.toml. When a matching task appears, roko delegates via A2A
- [ ] Delegation failure falls back to local agent
- [ ] `roko agent discover <url>` displays the external agent's capabilities

## Verify Recipe

```bash
# Spot-check with ripgrep / git on the touched files.
# Do NOT run cargo — the merge-back pipeline does that.
git diff --stat
```

Then check the commit-message trailer:

```bash
git log -1 --format=%B | rg "^tracker: INNO_39 done"
```

## Acceptance Criteria

- All Verify checkboxes pass on inspection.
- Configure an external agent in roko.toml. When a matching task appears, roko delegates via A2A
- Delegation failure falls back to local agent
- `roko agent discover <url>` displays the external agent's capabilities
- No files outside the Write Scope are modified.
- Commit message contains `tracker: INNO_39 done` trailer.
- Pre-commit pipeline (fmt + clippy + test) passes when run by the merge-back stage.

## Do NOT

- Compile or run tests inside the batch (`cargo check/test/clippy/build`).
- Touch files outside the Write Scope above.
- Bundle this batch with another tracker row, even if both touch the same file.
- Add `unwrap()`, `panic!()`, or `unreachable!()` to changed code.
- Add a second dispatch / prompt-assembly / chat-state path. See
  `context-pack/00-RULES.md` §"Universal anti-patterns".
- Refactor neighbours opportunistically. File a separate tracker row.
