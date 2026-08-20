# 142 — Knowledge Write-Back Proof (End-to-End Neuro Store Verification)

**Priority**: P2 — The knowledge write-back path (`RuntimeKnowledgeLifecycle::ingest_episode`) is called from the runner but has not been verified end-to-end; without proof, `roko knowledge query` may return empty results even after successful runs.
**Size**: S (1 day)
**Crates**: `crates/roko-cli/src/runner/event_loop.rs`, `crates/roko-neuro/src/`, `crates/roko-serve/src/routes/`
**Depends on**: None
**Sources**: `tmp/backlog/_mori-diffs-gaps.md` §F-3 (suggested 126)

---

## Background

The self-improving loop depends on knowledge accumulated from previous runs being injected into future runs. The chain is: successful task completion → `RuntimeKnowledgeLifecycle::ingest_episode` → `.roko/neuro/knowledge.jsonl` → `roko knowledge query` returns relevant entries → prompt assembly injects them → future agents benefit.

`RuntimeKnowledgeLifecycle::ingest_episode` is called from `event_loop.rs` at task completion, but no end-to-end verification confirms that:
1. Non-empty knowledge entries are actually written to `.roko/neuro/knowledge.jsonl`.
2. `roko knowledge query` returns those entries.
3. A subsequent runner dispatch includes a knowledge entry in its system prompt (with a knowledge ID in prompt diagnostics).

The write-back path may be silently failing, writing empty entries, or writing to a path that `roko knowledge query` does not read.

## Current State

- `crates/roko-cli/src/runner/event_loop.rs` — calls `RuntimeKnowledgeLifecycle::ingest_episode` at task completion.
- `crates/roko-neuro/src/` — implements `RuntimeKnowledgeLifecycle`; writes to `.roko/neuro/knowledge.jsonl`.
- `roko knowledge query` — queries the store.
- `GET /api/neuro/query` — HTTP equivalent.
- No proof script or integration test exercises this full chain.

## Implementation Plan

1. **Write a two-run proof script** at `tests/knowledge_proof/write_back.sh` (or as a Rust integration test):

   **Run 1 — Knowledge creation**:
   - Start with an empty `.roko/neuro/knowledge.jsonl`.
   - Run a simple plan with one task (e.g., "write a hello world function").
   - Assert: `wc -l .roko/neuro/knowledge.jsonl` > 0.
   - Assert: `roko knowledge query "hello world"` returns at least one result.

   **Run 2 — Knowledge injection**:
   - Run a second plan with a similar task.
   - Assert: `.roko/episodes.jsonl` entries for run 2 include a `knowledge_ids` field referencing the entry from run 1.
   - Assert: `GET /api/neuro/query?q=hello+world` returns the same entries.

2. **Fix bugs found during proof**: Common failure modes:
   - `ingest_episode` writes to a wrong path (fix the path constant).
   - `ingest_episode` writes entries with empty content (fix the episode-to-knowledge transformation).
   - `roko knowledge query` reads from a different path than the runner writes to (fix the path configuration).

3. **Add knowledge ID to prompt diagnostics**: The system prompt builder should include a `// Knowledge: <id>` comment at the start of injected knowledge sections. This makes it detectable in the agent's context (grep the prompt log from #149 for this marker).

4. **Minimum viable entry schema**: Define what a valid knowledge entry must contain for it to be useful: `{"id": "...", "content": "...", "task_id": "...", "run_id": "...", "confirmed_at": "...", "tier": "transient|working"}`. Entries missing required fields should be rejected at ingest time, not silently written.

## Acceptance Criteria

1. After run 1, `.roko/neuro/knowledge.jsonl` has at least one valid entry.
2. `roko knowledge query "<task-topic>"` returns the entry from run 1.
3. `GET /api/neuro/query?q=<topic>` returns the same entry.
4. Run 2 agent dispatch includes the knowledge entry ID in its prompt context (verifiable via prompt log from #149, or via episode metadata).
5. Entries with missing required fields are rejected at ingest, not written silently.

## Verification Checklist

- [ ] Run 1: verify `knowledge.jsonl` is non-empty after a successful task.
- [ ] `roko knowledge query "relevant topic"` returns results from run 1.
- [ ] Run 2: inspect episode metadata; verify `knowledge_ids` field references run 1 entries.
- [ ] Deliberately write a malformed entry; verify it is rejected, not silently accepted.
- [ ] `GET /api/neuro/query?q=test` returns results (assuming `roko serve` is running).

## Files to Modify

| File | Change |
|---|---|
| `crates/roko-cli/src/runner/event_loop.rs` | Verify `ingest_episode` is called correctly; fix if not |
| `crates/roko-neuro/src/` | Fix path constants, entry schema validation if needed |
| `crates/roko-compose/src/` | Add knowledge ID to prompt section diagnostics |
| `tests/knowledge_proof/` | New directory with two-run proof script |
