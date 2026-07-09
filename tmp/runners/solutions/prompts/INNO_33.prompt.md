# INNO_33: Implement tiered KnowledgeStore for cross-project sharing

## Tracker

- Issue tracker row: [`ISSUE-TRACKER.md#inno-33`](../ISSUE-TRACKER.md#inno-33)
- Source: `tmp/solutions/roko/tasks/11-INNOVATIONS.md` — Task 11.33
- Priority: **P3**
- Effort: 12 hours
- Depends on: `INNO_31` (source 11.31), `INNO_32` (source 11.32)

When this batch lands, the commit message MUST contain the trailer:

```
tracker: INNO_33 done
```

`bin/sync-tracker.py --apply` flips the corresponding `[ ]` to `[x]`.

## Problem

_(no context section in source)_

## Exact Changes

1. Define `TieredKnowledgeStore` wrapping:
   - Tier 0: project-specific (`.roko/neuro/knowledge.jsonl`)
   - Tier 1: domain-specific (`~/.roko/domains/{domain}/knowledge.jsonl`)
   - Tier 2: model meta-knowledge (`~/.roko/meta/model-knowledge.jsonl`)
2. Implement `query(topic: &str, domains: &[DomainTag]) -> Vec<KnowledgeEntry>`:
   - Always include Tier 0 and Tier 2.
   - Include Tier 1 only if domain tags match.
   - Rank by confidence, Tier 0 gets a small boost.
3. Implement conflict resolution: if entries conflict across tiers, use
   confidence score. If tied, prefer the more specific tier.

## Write Scope

- `crates/roko-neuro/src/lib.rs`

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

- [ ] Query from a Rust project returns Rust-specific Tier 1 knowledge
- [ ] Query from a TypeScript project does NOT return Rust-specific knowledge
- [ ] Model meta-knowledge (Tier 2) is available in all projects

## Verify Recipe

```bash
# Spot-check with ripgrep / git on the touched files.
# Do NOT run cargo — the merge-back pipeline does that.
git diff --stat
```

Then check the commit-message trailer:

```bash
git log -1 --format=%B | rg "^tracker: INNO_33 done"
```

## Acceptance Criteria

- All Verify checkboxes pass on inspection.
- Query from a Rust project returns Rust-specific Tier 1 knowledge
- Query from a TypeScript project does NOT return Rust-specific knowledge
- Model meta-knowledge (Tier 2) is available in all projects
- No files outside the Write Scope are modified.
- Commit message contains `tracker: INNO_33 done` trailer.
- Pre-commit pipeline (fmt + clippy + test) passes when run by the merge-back stage.

## Do NOT

- Compile or run tests inside the batch (`cargo check/test/clippy/build`).
- Touch files outside the Write Scope above.
- Bundle this batch with another tracker row, even if both touch the same file.
- Add `unwrap()`, `panic!()`, or `unreachable!()` to changed code.
- Add a second dispatch / prompt-assembly / chat-state path. See
  `context-pack/00-RULES.md` §"Universal anti-patterns".
- Refactor neighbours opportunistically. File a separate tracker row.
