# INNO_01: Create MemoryLayer struct wrapping three memory tiers

## Tracker

- Issue tracker row: [`ISSUE-TRACKER.md#inno-01`](../ISSUE-TRACKER.md#inno-01)
- Source: `tmp/solutions/roko/tasks/11-INNOVATIONS.md` — Task 11.1
- Priority: **P0**
- Effort: 4 hours
- Depends on: **none**

When this batch lands, the commit message MUST contain the trailer:

```
tracker: INNO_01 done
```

`bin/sync-tracker.py --apply` flips the corresponding `[ ]` to `[x]`.

## Problem

Agents are stateless between invocations. EpisodeLogger (`.roko/episodes.jsonl`),
KnowledgeStore (`.roko/neuro/knowledge.jsonl`), and PlaybookStore
(`.roko/learn/playbooks/`) all exist and are populated during runs, but nothing
unifies them or queries them at dispatch time.

Research: Mem0, JetBrains Research, ActiveContext (arxiv 2604.11462) -- vanilla
RAG fails for agentic use cases; agents need stateful persistence that recalls
context on demand, not just one-shot retrieval.

`EpisodeLogger` is at `crates/roko-learn/src/episode_logger.rs` with
`hdc_fingerprint: Option<String>` on episodes. `KnowledgeStore` is at
`crates/roko-neuro/src/knowledge_store.rs` with `ingest()`, `query()`,
`query_similar()`, and anti-knowledge support. `PlaybookStore` is at
`crates/roko-learn/src/playbook.rs` with `Playbook`, `PlaybookStep`.

## Exact Changes

1. Create `crates/roko-learn/src/memory_layer.rs`.
2. Define `MemoryLayer` struct holding owned instances (not references) of
   `EpisodeLogger`, `KnowledgeStore`, and `PlaybookStore`. Use `Arc` wrappers
   if shared ownership is needed.
3. Define `MemoryInjection` struct:
   ```rust
   pub struct MemoryInjection {
       pub playbooks: Vec<PlaybookEntry>,
       pub anti_patterns: Vec<String>,
       pub relevant_episodes: Vec<EpisodeSummary>,
       pub knowledge_entries: Vec<KnowledgeEntry>,
       pub total_tokens: usize,
   }
   ```
4. Define `EpisodeSummary` and `PlaybookEntry` as lightweight summary types
   (task_id, outcome, key_insight, confidence).
5. Implement `MemoryLayer::new(roko_dir: &Path) -> Result<Self>` that loads all
   three stores from their standard paths.
6. Stub `query_for_task(&self, ctx: &TaskContext) -> Result<MemoryInjection>`
   returning empty injection (implemented in Task 11.2).
7. Add `pub mod memory_layer;` to `crates/roko-learn/src/lib.rs`.
8. Ensure `roko-learn` can depend on `roko-neuro` for `KnowledgeStore` -- check
   `Cargo.toml` for existing dependency; add if missing.

## Write Scope

- `crates/roko-learn/src/lib.rs`
- `crates/roko-learn/Cargo.toml`

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

- [ ] `MemoryLayer::new()` succeeds on a `.roko/` directory with episode and knowledge data
- [ ] Unit test: construct MemoryLayer with empty stores, call `query_for_task`, get empty MemoryInjection
- [ ] `MemoryInjection` is importable from `roko_learn::memory_layer`

## Verify Recipe

```bash
# Spot-check with ripgrep / git on the touched files.
# Do NOT run cargo — the merge-back pipeline does that.
git diff --stat
```

Then check the commit-message trailer:

```bash
git log -1 --format=%B | rg "^tracker: INNO_01 done"
```

## Acceptance Criteria

- All Verify checkboxes pass on inspection.
- `MemoryLayer::new()` succeeds on a `.roko/` directory with episode and knowledge data
- Unit test: construct MemoryLayer with empty stores, call `query_for_task`, get empty MemoryInjection
- `MemoryInjection` is importable from `roko_learn::memory_layer`
- No files outside the Write Scope are modified.
- Commit message contains `tracker: INNO_01 done` trailer.
- Pre-commit pipeline (fmt + clippy + test) passes when run by the merge-back stage.

## Do NOT

- Compile or run tests inside the batch (`cargo check/test/clippy/build`).
- Touch files outside the Write Scope above.
- Bundle this batch with another tracker row, even if both touch the same file.
- Add `unwrap()`, `panic!()`, or `unreachable!()` to changed code.
- Add a second dispatch / prompt-assembly / chat-state path. See
  `context-pack/00-RULES.md` §"Universal anti-patterns".
- Refactor neighbours opportunistically. File a separate tracker row.
