# INNO_36: Define A2A core types

## Tracker

- Issue tracker row: [`ISSUE-TRACKER.md#inno-36`](../ISSUE-TRACKER.md#inno-36)
- Source: `tmp/solutions/roko/tasks/11-INNOVATIONS.md` — Task 11.36
- Priority: **P2**
- Effort: 8 hours
- Depends on: **none**

When this batch lands, the commit message MUST contain the trailer:

```
tracker: INNO_36 done
```

`bin/sync-tracker.py --apply` flips the corresponding `[ ]` to `[x]`.

## Problem

A2A v1.0 is stable with Signed Agent Cards, 150+ organizations, JSON-RPC + gRPC
bindings. Effectively unopposed as the cross-vendor agent bus.

## Exact Changes

1. Define `AgentCard` struct matching A2A v1.0 spec: `name`, `description`,
   `url`, `version`, `capabilities`, `skills`, `authentication`.
2. Define `A2ASkill`: `id`, `name`, `description`, `input_modes`, `output_modes`.
3. Define `A2ATask`: `id`, `status`, `messages`, `artifacts`.
4. Define `A2AArtifact`: `name`, `content_type`, `data`.
5. Implement JSON-RPC 2.0 request/response types for A2A methods:
   `tasks/send`, `tasks/get`, `tasks/cancel`, `tasks/sendSubscribe`.
6. Implement serde for all types, validated against A2A v1.0 spec.
7. AgentCard includes roko's 4 skills: code-implementation, code-review,
   gate-verification, knowledge-query.
8. Add `pub mod a2a;` to `crates/roko-core/src/lib.rs`.

## Write Scope

- `crates/roko-core/src/lib.rs`

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

- [ ] All types serialize to JSON matching the A2A v1.0 spec
- [ ] Round-trip test: serialize, deserialize, compare equality
- [ ] AgentCard includes roko's 4 skills

## Verify Recipe

```bash
# Spot-check with ripgrep / git on the touched files.
# Do NOT run cargo — the merge-back pipeline does that.
git diff --stat
```

Then check the commit-message trailer:

```bash
git log -1 --format=%B | rg "^tracker: INNO_36 done"
```

## Acceptance Criteria

- All Verify checkboxes pass on inspection.
- All types serialize to JSON matching the A2A v1.0 spec
- Round-trip test: serialize, deserialize, compare equality
- AgentCard includes roko's 4 skills
- No files outside the Write Scope are modified.
- Commit message contains `tracker: INNO_36 done` trailer.
- Pre-commit pipeline (fmt + clippy + test) passes when run by the merge-back stage.

## Do NOT

- Compile or run tests inside the batch (`cargo check/test/clippy/build`).
- Touch files outside the Write Scope above.
- Bundle this batch with another tracker row, even if both touch the same file.
- Add `unwrap()`, `panic!()`, or `unreachable!()` to changed code.
- Add a second dispatch / prompt-assembly / chat-state path. See
  `context-pack/00-RULES.md` §"Universal anti-patterns".
- Refactor neighbours opportunistically. File a separate tracker row.
