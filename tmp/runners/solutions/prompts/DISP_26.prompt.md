# DISP_26: Extract Context Bidding to roko-compose

## Tracker

- Issue tracker row: [`ISSUE-TRACKER.md#disp-26`](../ISSUE-TRACKER.md#disp-26)
- Source: `tmp/solutions/roko/tasks/03-INFERENCE-DISPATCH.md` — Task 3.26
- Priority: **P2**
- Effort: 5 hours
- Depends on: `DISP_24` (source 3.24)

When this batch lands, the commit message MUST contain the trailer:

```
tracker: DISP_26 done
```

`bin/sync-tracker.py --apply` flips the corresponding `[ ]` to `[x]`.

## Problem

orchestrate.rs contains `AttentionBidder` variants (Neuro, Task, Research context bidders) and a VCG auction system for prompt assembly. This logic determines which context sections get included in the system prompt and at what priority. The `vcg_allocate` function computes optimal allocation.

`roko-compose` already has `context_provider.rs` at `/Users/will/dev/nunchi/roko/roko/crates/roko-compose/src/context_provider.rs` (which has a `load_or_new` at line 462). The context bidding from orchestrate.rs should merge with the existing context provider infrastructure.

## Exact Changes

1. Identify the `AttentionBidder` types and `vcg_allocate` function in orchestrate.rs
2. Check if `context_provider.rs` already has equivalent functionality
3. If equivalent exists, verify feature parity and note any gaps
4. If not, extract the bidding types to `crates/roko-compose/src/context_bidding.rs`
5. Extract VCG auction to the same module or a submodule
6. Re-export from `crates/roko-compose/src/lib.rs`
7. Wire into `SystemPromptBuilder` as an optional enrichment step

## Design Guidance

Context bidding is about selecting the most valuable context sections for a prompt within a token budget. This naturally belongs in `roko-compose` next to `SystemPromptBuilder`. The VCG auction is an allocation mechanism -- it should be a generic utility that bidders plug into, not tied to specific bidder implementations.

## Write Scope

- `crates/roko-compose/src/context_bidding.rs`
- `crates/roko-compose/src/lib.rs`
- `crates/roko-cli/src/orchestrate.rs`

## Read-Only Context

The following files contain context that informs this change but **must
not** be modified by this batch. If you find yourself wanting to edit
them, file a follow-up tracker row instead.

- `tmp/runners/solutions/context-pack/00-RULES.md`
- `tmp/runners/solutions/context-pack/02-FILE-INVENTORY.md`
- `tmp/solutions/roko/tasks/03-INFERENCE-DISPATCH.md` (the full task list this came from)

## Verification Checklist

These criteria come straight from the source task. Tick each one before
opening the merge:

- [ ] Context bidding is accessible from `roko_compose`
- [ ] `SystemPromptBuilder` can optionally use context bidding for section selection

## Verify Recipe

```bash
# Spot-check with ripgrep / git on the touched files.
# Do NOT run cargo — the merge-back pipeline does that.
git diff --stat
```

Then check the commit-message trailer:

```bash
git log -1 --format=%B | rg "^tracker: DISP_26 done"
```

## Acceptance Criteria

- All Verify checkboxes pass on inspection.
- Context bidding is accessible from `roko_compose`
- `SystemPromptBuilder` can optionally use context bidding for section selection
- No files outside the Write Scope are modified.
- Commit message contains `tracker: DISP_26 done` trailer.
- Pre-commit pipeline (fmt + clippy + test) passes when run by the merge-back stage.

## Do NOT

- Compile or run tests inside the batch (`cargo check/test/clippy/build`).
- Touch files outside the Write Scope above.
- Bundle this batch with another tracker row, even if both touch the same file.
- Add `unwrap()`, `panic!()`, or `unreachable!()` to changed code.
- Add a second dispatch / prompt-assembly / chat-state path. See
  `context-pack/00-RULES.md` §"Universal anti-patterns".
- Refactor neighbours opportunistically. File a separate tracker row.
