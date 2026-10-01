+++
id = "bug-86117a"
kind = "bug"
title = "Graph prompts never include durable knowledge: PromptCache loads entries with an empty query that matches nothing"
status = "done"
triage = "verified"
severity = "p1"
goal = "learning"
size = "S"
subsystem = ["roko-cli/dispatch", "roko-execution"]
created = 2026-09-29
updated = 2026-10-01
last_verified = 2026-10-01
last_verified_rev = "479bec688"
source = "session:roko-b6 2026-09-29 direct-implementation batch"
discovered_from = "merge:feat/learning-completion-loops 189a14e65"
anchors = ["crates/roko-cli/src/dispatch/prompt_cache.rs::load_neuro_entries", "crates/roko-cli/src/dispatch/prompt_builder.rs::collect_neuro_knowledge_cached", "crates/roko-cli/src/dispatch/prompt_builder.rs::query_keywords", "crates/roko-cli/src/graph_execution/plan_runner.rs:867", "crates/roko-execution/src/prompt/cache.rs::load_neuro_entries", "crates/roko-neuro/src/knowledge_store/scoring.rs::score_entry_for_query"]
links = { depends_on = [], blocks = [], related = ["find-34a4b5", "reg-06ae9f"], supersedes = [], duplicate_of = "" }

[[verify]]
command = "grep -rqw 'fn cached_prompt_surfaces_matching_durable_knowledge' crates/roko-cli/src && cargo test -p roko-cli --lib cached_prompt_surfaces_matching_durable_knowledge"

[closed]
at = 2026-10-01
at_ts = "2026-10-01T12:49:55Z"
by = "coordinator (session 7622b882)"
forced = false
evidence = "Batch 20d gate on fae7133cd, re-checked with the clippy fix on 9f3c184c5 (MAIN 479bec688 has the same crates and portal): check --workspace --tests, nightly fmt and clippy -D warnings clean on roko-agent/cli/compose/core/execution/graph/neuro/runtime/serve; lib tests roko-cli 3282, roko-agent 2282, roko-core 1962, roko-serve 990, roko-compose 561, roko-graph 476, roko-runtime 288, roko-execution 245, roko-neuro 239 all pass; extras: codex/cursor/openai parity 4+4+4 (streaming tests no longer ignored), default_engine 1, C1 1, C7 2, bin 429, graph_task_dispatch loop 10/10, including cached_prompt_surfaces_matching_durable_knowledge and cached_knowledge_survives_a_long_domain_context. Merged 479bec688 (work/bug-86117a f124b144f + clippy fix 2729d5185)."
+++

## Problem

On `roko plan run`, no prompt ever carries a "Neuro knowledge" section, however many entries
`.roko/neuro/knowledge.jsonl` holds (43 hot entries of 205 in this checkout). `PromptCache::load`
fills `neuro_entries` with `KnowledgeStore::query("", 500)` (`dispatch/prompt_cache.rs:107`). An
empty topic scores every entry 0:
- `keyword_score` adds nothing for an empty topic and no terms
  (`roko-neuro/src/knowledge_store/scoring.rs:40-68`);
- the HDC probe for `""` is a fixed pseudo-random vector, so an ordinary entry's similarity to it
  stays near 0.5, below the 0.525 relevance threshold (about 5 sigma at 10,240 bits);
- `score_entry_for_query` drops every hit with `relevance_score <= QUERY_SCORE_FLOOR` (0.0,
  `scoring.rs:228`).

The cache is therefore empty, and `collect_neuro_knowledge_cached` returns `None` at its first line
(`prompt_builder.rs:2252`).

The cached keyword scorer is weak as well. `query_keywords` (`prompt_builder.rs:2465`) keeps every
token longer than two characters, including stopwords ("the", "and", "for", "with") and the role
name, and matches them as substrings (`haystack.contains`). So "the" also matches "other" and
"then". The cached episode section uses the same scorer, and it hits its cap on every task: all 269
records in `.roko/learn/retrieval-outcomes.jsonl` have `results_count` 5.

## Why it matters

Goal `learning`. 189a14e65 made gate-verified attempts grow durable knowledge
(`VerifiedKnowledgeSink`). Plan runs never read that knowledge back, so the loop stays open on the
read side. `diagnostics.knowledge_ids` (documented as "Neuro knowledge ids surfaced",
`prompt_builder.rs:1006`) also collects the episode section's ids (`:1685-1689`). On plan runs it
therefore holds only episode ids. Those ids reach `TaskCompleted.knowledge_ids` (episodes'
`knowledge_ids_injected`), `VerifiedKnowledgeSink`'s reinforcement (`context_entry_ids`) and the
RAG-10 retrieval telemetry. Related: find-34a4b5 (P0-07), reg-06ae9f.

## Where

- `crates/roko-cli/src/graph_execution/plan_runner.rs:867`: `PromptCache::load(workdir)`, passed
  to `SharedAgentFactory::new`, which picks `PromptAssembler::with_cache` (`dispatch/factory.rs:150`).
- `crates/roko-cli/src/dispatch/prompt_cache.rs::load_neuro_entries`: the empty query.
- `crates/roko-cli/src/dispatch/prompt_builder.rs`: `WorkdirKnowledgeSource::collect` (`:1976`),
  `collect_neuro_knowledge_cached` (`:2247`), `collect_episode_knowledge_cached` (`:2315`),
  `query_keywords` (`:2465`), and the id merge (`:1685`).
- `crates/roko-execution/src/prompt/cache.rs::load_neuro_entries` (`:110`): the same empty query in
  the `RuntimeServices` copy of the cache.

## Current state

Checked at 33e107da1. The uncached path (`collect_neuro_knowledge`, `:2049`) queries with the task
text and works. Graph dispatcher tests build `SharedAgentFactory::new(.., None)` (for example
`graph_task_dispatch.rs:5650`), so they take that path and never exercise the cache. The cache is
also never refreshed during a run: `SharedAgentFactory::update_prompt_cache` (`factory.rs:364`)
has no caller.

## Plan

1. Load every hot entry into the cache, not a query result: use `read_all()` and drop frozen and
   empty-content entries, as `query_hits_filtered` does. Fix both copies, or delete the
   `roko-execution` copy if nothing reads its `neuro_entries`.
2. Rank cached entries with the store's own scoring, or give `query_keywords` a stopword list and
   whole-word matching. Keep the three-entry cap of the uncached path.
3. Keep neuro ids and episode ids apart in `PromptDiagnostics`, or rename the field. Only knowledge
   entry ids may reach `VerifiedKnowledgeSink` and `knowledge_ids_injected`.
4. Optional: refresh the cache after a verified attempt admits knowledge, so later tasks of the same
   run can see it.

## Done when

- A plan-run prompt for a task whose text shares a content word with a stored entry contains that
  entry under "# Neuro knowledge", and `diagnostics.knowledge_ids` names it.
- An entry that shares only stopwords with the task is not included.
- The test `cached_prompt_surfaces_matching_durable_knowledge` (roko-cli lib) builds a
  `PromptAssembler::with_cache(PromptCache::load(..))` over a temporary store and asserts both
  points. The `[[verify]]` command passes.

## Notes

- `prompt_builder.rs` and `graph_task_dispatch.rs` change often; keep the change small.
- Check the result with a real `roko plan run`: the prompt, the episode's `knowledge_ids_injected`
  and `results_count` in `retrieval-outcomes.jsonl` (CLAUDE.md rule 3).
- 2026-10-01 (wk-childenv): Premise re-checked at `faa378453`: both caches still loaded `query("", 500)`, and
  `plan_runner.rs:923` (was :867) loads the cache for plan runs. Implemented on `work/bug-86117a` at `133c02093`;
  cargo verification is deferred to the batch check, and no real `roko plan run` has been done.
  - Load: both caches use the new `KnowledgeStore::hot_entries` (not frozen, non-blank content), which
    `query_hits_filtered` now shares.
  - Rank: per task, by the task's query text (id, title, role, description, acceptance, files). `query_keywords`
    drops stopwords, the cached neuro section matches whole words, and it keeps three entries. The stopword list
    also applies to the episode, playbook and group-knowledge scorers, which still match substrings.
  - Ids: `PromptDiagnostics.knowledge_ids` holds only the neuro section's ids, and the episode section's ids go to
    a new `episode_ids`.
  - Not done: plan step 4 (refreshing the cache after a run admits knowledge); the uncached path still ranks with
    `KnowledgeStore::query`.
  - Found while checking the test: the prompt composer may still drop the knowledge. Since dc418ae0a (2026-09-06),
    `foraging_prepass` (`roko-compose/src/prompt.rs`) keeps at most three optional sections: in density order,
    a candidate's density is never above the running mean, so the stop rule fires at the third. An implementer
    prompt has four (`conventions`, `tool_instructions`, `domain_context`, `anti_patterns`). Knowledge, episodes and
    playbooks all ride in `domain_context`, which loses whenever it is longer than `conventions` (about 1,230
    bytes), and the dropped section is missing from `dropped_sections` too. The test's domain context is about
    250 bytes, so it keeps the section.
- 2026-10-01 (wk-childenv): The foraging finding is bug-4aa696, fixed on this branch at `7531a3b39`.
  `cached_knowledge_survives_a_long_domain_context` (`381dbd45a`) covers the knowledge path with a `domain_context`
  longer than `conventions`.
