# INNO_65: Add ERC-8004 identity fields to agent configuration

## Tracker

- Issue tracker row: [`ISSUE-TRACKER.md#inno-65`](../ISSUE-TRACKER.md#inno-65)
- Source: `tmp/solutions/roko/tasks/11-INNOVATIONS.md` — Task 11.65
- Priority: **P3**
- Effort: 4 hours
- Depends on: `INNO_36` (source 11.36)

When this batch lands, the commit message MUST contain the trailer:

```
tracker: INNO_65 done
```

`bin/sync-tracker.py --apply` flips the corresponding `[ ]` to `[x]`.

## Problem

ERC-8004 is live on Ethereum mainnet (Jan 29, 2026). ~80-150K agents registered
by April 2026.

## Exact Changes

1. Add optional fields to agent config in roko.toml:
   ```toml
   [agent.identity]
   erc8004_id = "0x..."
   capabilities = ["code-implementation", "code-review"]
   reputation_tier = "verified"
   ```
2. If `erc8004_id` is set, include it in the A2A Agent Card.
3. Validate format (Ethereum address format).
4. Field is entirely optional.

## Write Scope

- `crates/roko-core/src/config/agent.rs`

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

- [ ] Agent config accepts `[agent.identity]` section without error
- [ ] ERC-8004 ID appears in A2A Agent Card when set
- [ ] Invalid Ethereum address format produces a config validation error
- [ ] Field is entirely optional; omitting it changes nothing

## Verify Recipe

```bash
# Spot-check with ripgrep / git on the touched files.
# Do NOT run cargo — the merge-back pipeline does that.
git diff --stat
```

Then check the commit-message trailer:

```bash
git log -1 --format=%B | rg "^tracker: INNO_65 done"
```

## Acceptance Criteria

- All Verify checkboxes pass on inspection.
- Agent config accepts `[agent.identity]` section without error
- ERC-8004 ID appears in A2A Agent Card when set
- Invalid Ethereum address format produces a config validation error
- Field is entirely optional; omitting it changes nothing
- No files outside the Write Scope are modified.
- Commit message contains `tracker: INNO_65 done` trailer.
- Pre-commit pipeline (fmt + clippy + test) passes when run by the merge-back stage.

## Do NOT

- Compile or run tests inside the batch (`cargo check/test/clippy/build`).
- Touch files outside the Write Scope above.
- Bundle this batch with another tracker row, even if both touch the same file.
- Add `unwrap()`, `panic!()`, or `unreachable!()` to changed code.
- Add a second dispatch / prompt-assembly / chat-state path. See
  `context-pack/00-RULES.md` §"Universal anti-patterns".
- Refactor neighbours opportunistically. File a separate tracker row.
