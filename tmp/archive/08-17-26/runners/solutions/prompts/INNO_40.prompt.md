# INNO_40: Implement CaMeL-style privileged/quarantined LLM split

## Tracker

- Issue tracker row: [`ISSUE-TRACKER.md#inno-40`](../ISSUE-TRACKER.md#inno-40)
- Source: `tmp/solutions/roko/tasks/11-INNOVATIONS.md` — Task 11.40
- Priority: **P1**
- Effort: 12 hours
- Depends on: **none**

When this batch lands, the commit message MUST contain the trailer:

```
tracker: INNO_40 done
```

`bin/sync-tracker.py --apply` flips the corresponding `[ ]` to `[x]`.

## Problem

Research: CaMeL (arxiv 2503.18813) -- solves 77% of AgentDojo with provable
security vs 84% undefended utility (7-point tax). Blocks ~67% of injections.
AutoInject (arxiv 2602.05746): 1.5B suffix-generator achieves 77.96% ASR.

AgentContract at `crates/roko-agent/src/safety/contract.rs` exists but falls
back to permissive default when YAML missing (CLAUDE.md: "Safety contracts
enforcement -- Partial").

## Exact Changes

1. Define `TrustDomain` enum: `Privileged`, `Quarantined`.
2. In the dispatch path, tag each LLM call with its trust domain:
   - Quarantined: agent implementation, tool output processing, user message
     handling, web content processing.
   - Privileged: gate evaluation, policy enforcement, system prompt assembly.
3. Enforce: quarantined LLM's chain-of-thought never influences privileged
   LLM's decisions (reasoning-blind classifier pattern).
4. Privileged calls should use a different model lineage from quarantined
   calls when feasible.
5. Log trust domain per LLM call in efficiency events.

## Write Scope

- `crates/roko-agent/src/safety/mod.rs`
- `crates/roko-agent/src/safety/contract.rs`

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

- [ ] Gate evaluation calls are tagged as Privileged
- [ ] Agent implementation calls are tagged as Quarantined
- [ ] If the agent used Claude, the gate judge uses a different model family (when configured)
- [ ] Trust domain is visible in episode logs

## Verify Recipe

```bash
# Spot-check with ripgrep / git on the touched files.
# Do NOT run cargo — the merge-back pipeline does that.
git diff --stat
```

Then check the commit-message trailer:

```bash
git log -1 --format=%B | rg "^tracker: INNO_40 done"
```

## Acceptance Criteria

- All Verify checkboxes pass on inspection.
- Gate evaluation calls are tagged as Privileged
- Agent implementation calls are tagged as Quarantined
- If the agent used Claude, the gate judge uses a different model family (when configured)
- Trust domain is visible in episode logs
- No files outside the Write Scope are modified.
- Commit message contains `tracker: INNO_40 done` trailer.
- Pre-commit pipeline (fmt + clippy + test) passes when run by the merge-back stage.

## Do NOT

- Compile or run tests inside the batch (`cargo check/test/clippy/build`).
- Touch files outside the Write Scope above.
- Bundle this batch with another tracker row, even if both touch the same file.
- Add `unwrap()`, `panic!()`, or `unreachable!()` to changed code.
- Add a second dispatch / prompt-assembly / chat-state path. See
  `context-pack/00-RULES.md` §"Universal anti-patterns".
- Refactor neighbours opportunistically. File a separate tracker row.
