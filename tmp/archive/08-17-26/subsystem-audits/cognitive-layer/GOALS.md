# Cognitive Layer: Goals

## End State

Durable knowledge store consulted at every agent dispatch. Dreams consolidate overnight. Episodes drive playbook evolution. Knowledge-informed routing replaces blind model selection.

## Key Properties

- **Knowledge-at-dispatch**: Every agent prompt enriched with relevant neuro entries (not just task context).
- **Dream consolidation on schedule**: Cron/daemon triggers NREM→REM→integration cycle, not just manual `/dream`.
- **Episode → Playbook loop**: Successful episodes auto-promote to playbooks; failed ones feed anti-patterns.
- **HDC similarity in prompt assembly**: Fingerprint-based retrieval for "tasks like this one" context injection.
- **Tier progression**: Knowledge entries progress Transient→Working→Consolidated→Persistent based on confirmation count and distinct context count (not "cold→warm→hot" — those are not the actual tier names).
- **Cross-agent knowledge sharing**: Agent A's discoveries available to Agent B via shared neuro store.

## What Exists Today

- roko-neuro: durable knowledge store (4,047 LOC), distillation (950 LOC), tier progression (2,322 LOC) — all wired
- roko-dreams: hypnagogia, imagination, cycle (~13,600 LOC total including phase2) — `maybe_auto_dream()` fires at plan completion, no background cron trigger
- HDC fingerprint per-episode: wired — `attach_episode_hdc_fingerprint()` called in orchestrate.rs:9685
- Playbook store queries at dispatch time → system prompt
- Knowledge **is** consulted for CascadeRouter scoring: `knowledge_routing_boost()` adjusts candidate scores, `build_knowledge_routing_advice()` annotates the routing explanation (orchestrate.rs:14138, 14198)
- Knowledge query via `roko knowledge query` CLI command

## From v2 UX Showcase (9 Scenarios)

- **KnowledgeCard injection** (pipeline, tournament, incident, architect, follow, pair): Inline card showing neuro store hits with: score (0.92), source path (neuro:perf-patterns/cache.md, playbook:rs256-migration.md, episode:#3471), text snippet. "N hits · neuro store" header.
- **Knowledge panel** (right rail, all): Per-hit display with score pill (color-coded: >85 green, >70 teal, else dim), source path, text. Footer: "injected via SystemPromptBuilder · L7 of 9". Empty state: "no neuro hits this turn".
- **Per-scenario knowledge sources**: Different stores queried depending on scenario — playbooks for P1 triage, neuro entries for code patterns, episodes for past similar work.
- **Cross-agent knowledge** (pair): "Pair convergence was fast because both agents had access to same trace span knowledge" — neuro entries shared across pair agents.

### Data Feeds Required
- `KnowledgeQueryHit` (actual struct name in `roko-neuro/src/knowledge_store.rs`) — `total_score` (f64), `entry` (KnowledgeEntry with `tier: KnowledgeTier` = Transient/Working/Consolidated/Persistent), `breakdown` (keyword_score, effective_confidence, recency_factor, emotional_boost, hdc_similarity)
- `InjectionMetadata` — layer_number (e.g. L7), total_layers (9), injected_by (SystemPromptBuilder)
- `KnowledgeQuery` — task_context → relevant hits, query latency

## Gap

- Knowledge IS consulted during CascadeRouter model selection via `knowledge_routing_boost()` — but feedback loop from routing outcomes back to knowledge store is missing
- Dream cycle has no cron/daemon trigger (fires only on plan completion via `maybe_auto_dream()`, or conductor critical patterns, or manual `roko knowledge dream run`)
- HDC similarity in prompt assembly: the `hdc` feature must be enabled; when disabled (default), HDC contribution is 0
- No automated episode → playbook promotion pipeline (tier_progression runs D1→D2→D3 stages, but it is not automatically triggered after each plan)
- No cross-agent knowledge sharing protocol (shared store on disk, but no sync/merge protocol between concurrent agents)

---

## Sources

| File | What was checked |
|---|---|
| `crates/roko-neuro/src/lib.rs` | `KnowledgeTier` enum names (Transient/Working/Consolidated/Persistent), `KnowledgeEntry` struct |
| `crates/roko-neuro/src/knowledge_store.rs` | `KnowledgeQueryHit` struct, `KnowledgeQueryBreakdown` breakdown fields, `query_hits()` |
| `crates/roko-learn/src/episode_logger.rs` | `Episode.hdc_fingerprint: Option<String>` field |
| `crates/roko-cli/src/orchestrate.rs` | `attach_episode_hdc_fingerprint()` at 9685; `knowledge_routing_boost()` at 14138; `build_knowledge_routing_advice()` at 14198; `maybe_auto_dream()` at 7316 |
| `crates/roko-dreams/src/runner.rs` | `DreamRunner`, `maybe_auto_dream` logic, `load_latest_dream_report()` |
