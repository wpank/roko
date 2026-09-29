+++
id = "bug-86117a"
kind = "bug"
title = "Graph prompts never include durable knowledge: PromptCache loads entries with an empty query that matches nothing"
status = "open"
triage = "verified"
severity = "p1"
goal = "learning"
size = "S"
subsystem = ["roko-cli/dispatch", "roko-execution"]
created = 2026-09-29
updated = 2026-09-29
last_verified = 2026-09-29
last_verified_rev = "33e107da1"
source = "session:roko-b6 2026-09-29 direct-implementation batch"
discovered_from = "merge:feat/learning-completion-loops 189a14e65"
anchors = ["crates/roko-cli/src/dispatch/prompt_cache.rs::load_neuro_entries", "crates/roko-cli/src/dispatch/prompt_builder.rs::collect_neuro_knowledge_cached", "crates/roko-cli/src/dispatch/prompt_builder.rs::query_keywords", "crates/roko-cli/src/graph_execution/plan_runner.rs:867", "crates/roko-execution/src/prompt/cache.rs::load_neuro_entries", "crates/roko-neuro/src/knowledge_store/scoring.rs::score_entry_for_query"]
links = { depends_on = [], blocks = [], related = ["find-34a4b5", "reg-06ae9f"], supersedes = [], duplicate_of = "" }

[[verify]]
command = "grep -rqw 'fn cached_prompt_surfaces_matching_durable_knowledge' crates/roko-cli/src && cargo test -p roko-cli --lib cached_prompt_surfaces_matching_durable_knowledge"
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
