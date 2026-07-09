# D — Distillation + Tier Progression (Doc 12)

Parity analysis of `docs/06-neuro/12-4-tier-distillation-pipeline.md` vs the
actual codebase.

---

## D.01 — D1 stage: episode → insight extraction

**Status**: PARTIAL
**Severity**: MEDIUM
**Doc claim**: Doc 12 §"Stage D1: Episodes → Insights" — D1 uses an LLM to extract `Observations`, `Patterns`, `Dangers`, `Contradictions` from episode transcripts, runs async via `spawn_episode_distillation`, and yields `KnowledgeEntry` items into a `NeuroStore` at `Transient` tier with confidence `0.3-0.6`.
**Reality**: Two disjoint implementations. (a) `crates/roko-neuro/src/distiller.rs:81-94` — `Distiller::distill(episodes)` calls the Claude Haiku backend, extracts `insight/heuristic/warning/causal_link/strategy_fragment` via `build_prompt` at `:320-340`, parses `<|json|>` envelope via `parse_distillation_response` at `:393-399`, and produces `KnowledgeEntry` values with `DEFAULT_CONFIDENCE = 0.75` at `:27`. (b) `crates/roko-neuro/src/tier_progression.rs:312-387` — `TierProgression::extract_insights(episodes, candidate_patterns)` runs a **deterministic trigram miner** (no LLM) that emits `InsightRecord { antecedent, consequent, support_count, confidence }` only when `support_count >= min_support` (default 3 at `:24`). Neither path sets initial tier to `Transient` — `KnowledgeEntry::from(&InsightRecord)` at `tier_progression.rs:482` hardcodes `tier: KnowledgeTier::Consolidated`. The doc's claim of "confidence 0.3-0.6" does not match either implementation.
**Fix sketch**: Doc should distinguish the two D1 paths (LLM-based `Distiller` vs deterministic `TierProgression::extract_insights`) and update the confidence range to match code (`DEFAULT_CONFIDENCE = 0.75` for LLM path, `support_count / antecedent_episode_count` ratio for trigram path). Drop the "Transient" initial-tier claim or add the tier-assignment override.

---

## D.02 — D2 stage: insight → heuristic with `5+ @ ≥0.7` threshold

**Status**: PARTIAL
**Severity**: LOW
**Doc claim**: Doc 12 §"Stage D2: Insights → Heuristics" + §"Promotion Criteria" — D2 filters clusters with `≥ 5 members` **and** `mean confidence ≥ 0.7`, **plus** `≥ 2 distinct contexts` (cross-validation) **and** no AntiKnowledge contradictions. All four must pass.
**Reality**: `crates/roko-neuro/src/tier_progression.rs:391-421` — `promote_heuristics()` filters insights where `source_episodes.len() >= self.min_heuristic_support && confidence >= self.min_confidence`. Constants at `:25-26`: `DEFAULT_MIN_HEURISTIC_SUPPORT = 5`, `DEFAULT_MIN_CONFIDENCE = 0.7`. Those two thresholds match the doc exactly. Cross-validation (≥2 distinct contexts) and AntiKnowledge checks are **not** implemented — `rg cross_validation_check|anti_knowledge_check crates/` returns zero matches.
**Fix sketch**: Update doc §"Promotion Criteria" to reflect that only the first two criteria are enforced; move the cross-validation and AntiKnowledge rows into "Missing" or the "Implementation Details" designed-but-unimplemented block (already present at lines 775-781 of the doc). The `5+ @ ≥0.7` core threshold is correctly encoded.

---

## D.03 — D3 stage: heuristic → PLAYBOOK.md compilation

**Status**: PARTIAL
**Severity**: MEDIUM
**Doc claim**: Doc 12 §"Stage D3: Heuristics → PLAYBOOK.md" + §"Playbook Format" — D3 compiles validated Heuristics into Markdown and writes to `.roko/neuro/PLAYBOOK.md`. Example output shows sections like `## Rust Development Rules` grouped by topic, rules numbered and tagged with `Confidence`, `Support`, `Evidence`.
**Reality**: `crates/roko-neuro/src/tier_progression.rs:425-437` — `compile_playbook()` returns `PlaybookCompilation { markdown, rules }` where `markdown` is produced by `render_playbook_markdown()` at `:739-774`. The rendered format at `:744-749` is `# PLAYBOOK\n\nGenerated from {} insights and {} heuristics.\n\n## Action Rules\n\n` followed by numbered entries — **not** the topic-grouped "Agent Playbook" format the doc shows. `write_playbook()` at `:445-458` writes to the caller-supplied `path`, but no caller passes `.roko/neuro/PLAYBOOK.md` — `rg write_playbook crates/` returns one definition and zero call sites. The canonical playbook path in `crates/roko-fs/src/layout.rs:219-223` is `.roko/memory/playbook.toml` (TOML, not Markdown), a different artefact (`PlaybookStore` in `roko-dreams`).
**Fix sketch**: Either (a) replace the doc's example output with the real `render_playbook_markdown` format (flat `## Action Rules` list, JSON-embedded rule blocks), or (b) change the renderer to emit topic groupings matching the doc. Fix the path claim: the default output path is not hard-coded to `.roko/neuro/PLAYBOOK.md` — `write_playbook` is parametric and currently uncalled.

---

## D.04 — `Distiller` struct and backend constructors

**Status**: DONE
**Severity**: —
**Doc claim**: Doc 12 §"The DistillationBackend Trait" — `Distiller` is the default implementation, uses Claude Haiku by default, prompted to extract `Observations/Patterns/Dangers/Contradictions`.
**Reality**: `crates/roko-neuro/src/distiller.rs:42-95` defines `Distiller { backend: Arc<dyn DistillationBackend> }`. `DEFAULT_MODEL = "claude-haiku-3-5"` at `:25`. Constructors: `with_claude(api_key)` at `:51-54`, `with_claude_model(api_key, model)` at `:58-61`, `with_backend(backend)` at `:64-67`. `ClaudeDistillationBackend::new` at `:103-111` wires a `ClaudeAgent` with the distillation system prompt. The prompt categories at `:332-338` are `insight/heuristic/warning/causal_link/strategy_fragment` — slight naming difference from doc's `Observations/Patterns/Dangers/Contradictions`, but equivalent knowledge kinds.
**Notes**: Doc example shows the trait method as `async fn distill(&self, episode: &Episode) -> Result<Vec<KnowledgeEntry>>` (singular) — the real trait method at `distiller.rs:32-38` is `async fn complete(&self, prompt: &str) -> Result<String>`. The `distill` method belongs to `Distiller` itself at `:81-94`, which takes `&[Episode]` (a slice). Minor doc drift only.

---

## D.05 — `DistillationBackend` trait (single `complete` method)

**Status**: PARTIAL
**Severity**: LOW
**Doc claim**: Doc 12 §"The DistillationBackend Trait" — trait has one method: `async fn distill(&self, episode: &Episode) -> Result<Vec<KnowledgeEntry>>`.
**Reality**: `crates/roko-neuro/src/distiller.rs:30-38` defines `pub trait DistillationBackend: Send + Sync + std::fmt::Debug` with **two** methods: `async fn complete(&self, prompt: &str) -> Result<String>` and `fn model(&self) -> &str`. The backend returns raw text; `Distiller::distill` at `:81-94` owns prompt construction and response parsing, not the backend. `#[async_trait]` at `:30` matches doc's `async fn` style.
**Fix sketch**: Rewrite doc §"The DistillationBackend Trait" to show the `complete(prompt) -> String` + `model()` signature. Clarify that structured-output parsing lives in `Distiller::distill`, not the trait.

---

## D.06 — `TierProgression` struct (thresholds + defaults)

**Status**: PARTIAL
**Severity**: LOW
**Doc claim**: Doc 12 §"TierProgression Orchestrator" — struct holds `knowledge_store: Arc<Mutex<KnowledgeStore>>` and `pattern_miner: Arc<PatternMiner>` and exposes `analyze`, `extract_insights`, `promote_heuristics`, `compile_playbook`, `replay_heuristics`.
**Reality**: `crates/roko-neuro/src/tier_progression.rs:166-172` — `TierProgression { min_support, min_heuristic_support, min_confidence, playbook_limit }`. All four are usize/f64 thresholds, **not** `Arc<Mutex<KnowledgeStore>>` or `Arc<PatternMiner>`. Defaults at `:174-183`: `min_support=3, min_heuristic_support=5, min_confidence=0.7, playbook_limit=12`. Constructor at `:188-203`. The struct does **not** own a store or miner — it is pure and stateless, and callers pass episodes in via `analyze(&[Episode])`. Methods `analyze` at `:207`, `extract_insights` at `:312`, `promote_heuristics` at `:391`, `compile_playbook` at `:425`, `replay_heuristics` at `:267` all exist with shapes close to doc.
**Fix sketch**: Rewrite doc §"TierProgression Orchestrator" to show the actual four-field threshold struct and remove the `Arc<Mutex<KnowledgeStore>>` / `Arc<PatternMiner>` claims. Note that `TierProgression` is `Copy` and holds only thresholds; episodes flow in via method arguments.

---

## D.07 — `TierProgressionDecision` enum (4 variants)

**Status**: PARTIAL
**Severity**: MEDIUM
**Doc claim**: Audit item lists 4 variants: `Promote`, `Demote`, `Retain`, `Retire`.
**Reality**: `crates/roko-neuro/src/tier_progression.rs:136-146` defines `enum TierProgressionDecision` with 4 variants but different names: `Promote(KnowledgeTier)`, `Demote(KnowledgeTier)`, `ReviewExpiry`, `NoChange`. Helpers at `:148-163`: `tier()` returns `Some(tier)` for Promote/Demote, `None` for the rest; `needs_expiry_review()` matches `ReviewExpiry`. Wired into orchestrator at `crates/roko-cli/src/orchestrate.rs:12932-12948`: `evaluate_tier_progression` returns one of the four variants, used to mutate `entry.tier` or append the `expiry-review` tag.
**Fix sketch**: The audit checklist's variant names (`Retain`, `Retire`) don't match the code. Likely the doc's *intent* was `ReviewExpiry ≈ Retire` and `NoChange ≈ Retain`. If the checklist is canonical, rename the Rust variants; if the code is canonical, update the checklist. Either way, doc 12 itself does not enumerate these four variants — it should add a §"TierProgressionDecision" block matching reality.

---

## D.08 — `InsightRecord`, `HeuristicRule`, `PlaybookCompilation` data shapes

**Status**: PARTIAL
**Severity**: LOW
**Doc claim**: Doc 12 §"InsightRecord" (lines 70-78), §"HeuristicRule" (lines 99-109), §"Playbook Compilation" (lines 146-152) — `InsightRecord { pattern, support, confidence, source_episodes }`; `HeuristicRule { rule, support, confidence, source_insights }`; `PlaybookCompilation { title, rules, markdown }`.
**Reality**: Shapes match conceptually but field names drift from doc.
- `InsightRecord` at `crates/roko-neuro/src/tier_progression.rs:32-51` has **9** fields: `id`, `antecedent: Vec<String>`, `consequent: String`, `support_count`, `antecedent_episode_count`, `confidence`, `first_seen_ms`, `last_seen_ms`, `source_episodes`. Doc's `pattern: String` is split into `antecedent + consequent`; `support` → `support_count`.
- `HeuristicRule` at `:71-98` has **12** fields: `id`, `insight_id`, `title`, `when_clause`, `then_clause`, `confidence`, `confirmations`, `first_seen_ms`, `last_seen_ms`, `source_episodes`, `source_model: Option<String>` (with `#[serde(default, skip_serializing_if = "Option::is_none")]` at `:93`), `model_generality: f64` (with `#[serde(default = "default_model_generality")]` at `:96`). Doc's `rule: String` is split into `title + when_clause + then_clause`; `support` → `confirmations`; doc's `source_insights` is not present (code uses a single `insight_id: String` for a single source).
- `PlaybookCompilation` at `:116-122` has exactly **2** fields: `markdown: String`, `rules: Vec<HeuristicRule>`. Doc claims a third field `title: String` (line 148), which does not exist in the struct.
**Fix sketch**: Replace doc struct blocks with the real Rust definitions (copy from `tier_progression.rs:31-122`). Drop `PlaybookCompilation.title` from doc. Explain the antecedent/consequent split for `InsightRecord` and the when/then split for `HeuristicRule`. Note that `HeuristicRule` carries model-scope metadata (`source_model`, `model_generality`) that the doc block omits entirely — this powers `applies_to_model()` at `:109-113`.

---

## D.09 — `spawn_episode_distillation` hook wiring

**Status**: DONE
**Severity**: —
**Doc claim**: Doc 12 §"Episode Distillation" (lines 32-45) — `spawn_episode_distillation(episode: Episode, distiller: Arc<dyn DistillationBackend>, store: Arc<Mutex<KnowledgeStore>>) -> JoinHandle<Result<()>>` runs on a background task, triggered when an episode finishes.
**Reality**: `crates/roko-neuro/src/episode_completion.rs:16-22` defines `pub fn spawn_episode_distillation(workdir: PathBuf, episode: Episode)` — signature differs from doc (takes `workdir + episode`, not `episode + distiller + store`). Returns `()` via `tokio::spawn`, not a typed `JoinHandle<Result<()>>`. Internally the inner `distill_episode` at `:24-54` constructs `Distiller::with_claude(api_key)` at `:33` and `KnowledgeStore::for_workdir(workdir)` at `:43`, then writes entries via `task::spawn_blocking` at `:44-51`. Called from **five** sites (verified via `rg spawn_episode_distillation crates/`):
- `crates/roko-cli/src/agent_exec.rs:206` (inside `set_episode_completion_hook` closure)
- `crates/roko-cli/src/main.rs:4769` (inside a learning-runtime hook)
- `crates/roko-cli/src/orchestrate.rs:696` (via `install_episode_distillation_hook` at `:693-698`)
- `crates/roko-cli/src/run.rs:654` (inside the run-command hook)
- `crates/roko-serve/src/dispatch.rs:2098` (called directly after `logger.append` at `:2090`)
Wired through the CLI run loop, the orchestrator, and the serve dispatch — genuine end-to-end hook.
**Fix sketch**: Rewrite the doc's signature block to show the real `(PathBuf, Episode)` params and the `()` return type, and note that the distiller/store are constructed inside the spawned task from `ANTHROPIC_API_KEY` plus the workspace dir, not passed in by the caller.
**Notes**: The behaviour (background distillation, silent failure via `tracing::warn!` at `:19`) matches the doc's "avoid blocking the agent's main execution loop". If `ANTHROPIC_API_KEY` is unset or empty (`:25-31`), the hook returns `Ok(())` without calling the LLM — a soft no-op. The hook fires on every completed episode (no batching), so it is effectively D1 per-episode, not curriculum-ordered (see D.14 for the missing scheduler).

---

## D.10 — DreamCycle + orchestrator integration

**Status**: DONE
**Severity**: —
**Doc claim**: Doc 12 §"Integration with Dreams" (lines 263-272) — Dreams cycle drives the pipeline during idle time: NREM replay, D1+D2 on replayed episodes, pruning, D3 recompile. `DreamsDistillationTrigger` is the designed API.
**Reality**: `crates/roko-dreams/src/cycle.rs:398-430` — inside `DreamCycle::run_budgeted` (not `consolidate`) the code at `:428-430` constructs `TierProgression::default()`, calls `progression.analyze(&batch)` at `:429`, and runs `progression.replay_heuristics(&mut analysis, &batch)` at `:430`. The resulting `TierProgressionReport` is stored as the `analysis: TierProgressionReport` field of `DreamCycleReport` (`:67-79`), emitted when the cycle writes its summary at `:511`. The `review_insights_from_heuristics` helper at `:2044-2047` ingests that report to synthesize review-mode knowledge entries. A second wiring site: `crates/roko-dreams/src/runner.rs:541-546` — `DreamRunner::replay_insights(&[Episode])` calls `NeuroTierProgression::default().analyze(&replay.episodes)` and returns `report.insights`. Tier-feedback from gate verdicts into the `KnowledgeStore` is wired at `crates/roko-cli/src/orchestrate.rs:12912-12956` via `apply_knowledge_tier_feedback`, triggered from `:5322` after each task completes.
**Notes**: The `DreamsDistillationTrigger` type, `extract_warnings`, `HdcClusterer`, `cross_validation_check`, and `anti_knowledge_check` from doc §"Implementation Details" and §"Current Status and Gaps" are **not** implemented (`rg DreamsDistillationTrigger|HdcClusterer|extract_warnings|cross_validation_check|anti_knowledge_check crates/` returns zero matches). The doc itself marks those as "designed above" (lines 285-521) and flags them as Missing at lines 775-781 — accurate self-annotation. Distillation scheduling (`DistillationScheduler`, `D1Policy`, `D2Policy`, `D3Policy`, curriculum ordering, token budget) is also unimplemented (`rg D1Policy|D2Policy|D3Policy|DistillationScheduler crates/` returns zero matches). The `roko-learn` side has `UpdateFrequency { distiller_every_n: 50 }` at `crates/roko-learn/src/runtime_feedback.rs:175, 213`, but the `distiller_due` check at `:862` drives `append_cfactor_snapshot`, not an actual distillation invocation — the counter is wired for cadence metrics only. See D.11–D.14 for each designed-but-unimplemented PRD block.

---

## D.11 — `extract_warnings` deterministic D1 warning extraction

**Status**: NOT DONE
**Severity**: MEDIUM
**Doc claim**: Doc 12 §"Implementation Details: D1 Warning Extraction" (lines 285-357) — D1 should synthesize `Warning` `KnowledgeEntry` values from structural analysis of episode data (gate failures, excessive retries) without an LLM call. Example signature: `pub fn extract_warnings(episode: &Episode) -> Vec<KnowledgeEntry>`, designed to be called "inside `spawn_episode_distillation()` alongside the LLM-based insight extraction" (line 357). Uses `episode.gate_results` and iterates `episode.turns.filter(|t| t.is_retry)`.
**Reality**: No `extract_warnings` function exists (`rg extract_warnings crates/` returns zero hits). The `WarningCategory` enum (`GateFailure / PerformanceRegression / RecurringError / TimeoutPattern`) is not defined anywhere. The assumed Episode shape does not match reality either: `crates/roko-learn/src/episode_logger.rs:169-250` defines `Episode` with `gate_verdicts: Vec<GateVerdict>` (not `gate_results`) and `turns: u64` (a scalar counter, not `Vec<Turn>` with an `is_retry` boolean). The Warning path today is LLM-driven only — `Distiller::distill` prompts Haiku to return `warning` entries inside the shared envelope at `distiller.rs:335` ("warning: a recurring failure mode, guardrail, or risk to avoid"). Warning half-life is set to 7 days via `WARNING_HALF_LIFE_DAYS: f64 = 7.0` at `roko-neuro/src/lib.rs:33`, so the infrastructure for the output type is ready — only the deterministic extractor is missing.
**Fix sketch**: Either (a) build the extractor: iterate `episode.gate_verdicts` looking for `!verdict.passed` and emit a `Warning` per failed gate, then detect retry patterns via `episode.turns > 2` (the scalar count, not a retry flag since the detailed turn struct no longer exists), and splice the resulting entries into `episode_completion::distill_episode` before the `KnowledgeStore.add` loop; or (b) update the doc to drop the deterministic extractor and re-describe the current LLM-only warning path, including the `turns: u64` / `gate_verdicts` shape. The doc's example also assumes a `KnowledgeEntry::default()` impl, which does not exist (`KnowledgeEntry` has no `Default` derive in `roko-neuro/src/lib.rs:186-243`).

---

## D.12 — `HdcClusterer` HDC-based D2 clustering

**Status**: NOT DONE
**Severity**: LOW
**Doc claim**: Doc 12 §"D2: HDC-based clustering algorithm" (lines 359-405) — D2 should offer an alternative to tag/text-based pattern mining by grouping Insights by HDC role-filler similarity. Signature: `pub struct HdcClusterer { min_cluster_size: usize, membership_threshold: f32, max_clusters: usize }` with defaults `5 / 0.53 / 50`. Algorithm: k-medoids (PAM) over Hamming distance, `k = sqrt(n/2)` capped at `max_clusters`, delegating to `roko-learn::hdc_clustering::k_medoids_pam`. Output: `InsightCluster { centroid, members, cohesion, mean_confidence }`.
**Reality**: No `HdcClusterer` or `InsightCluster` type exists (`rg HdcClusterer|InsightCluster crates/` returns zero hits). The D2 path in `tier_progression.rs:391-421` (`promote_heuristics`) uses pure filter-and-count on the `InsightRecord` antecedent/consequent strings and the `source_episodes.len() >= min_heuristic_support && confidence >= min_confidence` predicate — no vector embeddings. The `KnowledgeEntry.hdc_vector: Option<Vec<u8>>` field exists (`roko-neuro/src/lib.rs:242`) and both `InsightRecord` → `KnowledgeEntry` and `HeuristicRule` → `KnowledgeEntry` currently set it to `None` (`tier_progression.rs:485, 514`). The `roko-learn::hdc_clustering` module the doc would delegate to also does not exist (`rg k_medoids_pam crates/` returns zero matches). The tier-progression pipeline is therefore text-trigram based end-to-end; HDC similarity is wired only in `roko-neuro::knowledge_store` retrieval (feature-gated `#[cfg(feature = "hdc")]` at `lib.rs:371`), not in D2 promotion.
**Fix sketch**: Either (a) flag this PRD block as aspirational and move it from "Implementation Details" to an "Open Problems" section in the doc, or (b) commit to building it: plumb a `hdc_vector` onto insights during D1 (via `roko-index::hdc` which does exist), add a `roko-learn::hdc_clustering` submodule with k-medoids, and add a feature-flagged `HdcClusterer` variant of `promote_heuristics`.

---

## D.13 — Cross-validation + AntiKnowledge promotion gates

**Status**: NOT DONE
**Severity**: MEDIUM
**Doc claim**: Doc 12 §"Promotion criteria: cross-validation and AntiKnowledge check" (lines 407-459) — Before promoting a cluster to a `HeuristicRule`, D2 must verify two additional gates: (1) `cross_validation_check(cluster, entries, min_contexts) -> bool` enforces at least 2 distinct `context:*` tags across cluster members; (2) `anti_knowledge_check(cluster, anti_entries, threshold) -> bool` returns false if any active `AntiKnowledge` entry's HDC vector is similar to the cluster centroid (>0.526 Hamming). "Full promotion gate: Requires (1) >= 5 members, (2) mean confidence >= 0.7, (3) >= 2 distinct contexts, (4) no AntiKnowledge contradictions. All four must pass."
**Reality**: Neither gate is implemented (`rg cross_validation_check|anti_knowledge_check crates/` returns zero hits). The current `promote_heuristics` at `tier_progression.rs:391-421` enforces only the first two conditions — `source_episodes.len() >= min_heuristic_support` (5) and `confidence >= min_confidence` (0.7). `KnowledgeKind::AntiKnowledge` exists at `lib.rs:50`, `KnowledgeEntry::refutation_warning` at `lib.rs:247-271` formats the warning text for an AntiKnowledge entry, and `refuted_insight_id`/`refutation_evidence` fields are on the base entry — but no code consults these during heuristic promotion. There is no `context:*` tag convention enforced anywhere in the codebase (`rg "context:" crates/ --type rust` shows unrelated `RokoContext` types, not a tag namespace).
**Fix sketch**: To wire (1): add a `min_distinct_contexts: usize` field to `TierProgression` (default 2), derive a context key from each source episode (e.g. `episode.agent_template` + `episode.task_id` prefix) inside `promote_heuristics`, and drop clusters where `distinct_contexts.len() < min_distinct_contexts`. To wire (2): after computing the candidate `HeuristicRule`, scan the current `KnowledgeStore` for active `KnowledgeKind::AntiKnowledge` entries that tag or reference the same insight and veto the promotion. Until both are wired, the doc's "all four must pass" claim at line 459 and the companion table at lines 117-118 should be flagged as aspirational.

---

## D.14 — `DistillationScheduler` + `DistillationQualityReport`

**Status**: NOT DONE
**Severity**: LOW
**Doc claim**: Doc 12 §"Distillation Quality Metrics" (lines 569-645) + §"Distillation Scheduling" (lines 649-762) — The pipeline should report `DistillationQualityReport { timestamp, d1: D1Metrics, d2: D2Metrics, d3: D3Metrics, pipeline_health }` logged to `.roko/learn/distillation-quality.jsonl`. Scheduling should be controlled by `DistillationScheduler { d1_policy, d2_policy, d3_policy, hourly_token_budget, tokens_used_this_hour }` with `D1Policy { run_after_episode, batch_size, min_episode_quality, curriculum_ordering }`, `D2Policy { min_interval, min_new_insights, dreams_only }`, `D3Policy { min_interval, min_new_heuristics, dreams_only }`. Defaults documented: D1 batch size 1, D2 min_interval 6h, D3 min_interval 24h, hourly token budget 50,000. Also: curriculum ordering by `1.0 - (gate_pass_count / total_gates)`.
**Reality**: No scheduler or metrics struct exists (`rg DistillationScheduler|D1Policy|D2Policy|D3Policy|DistillationQualityReport|D1Metrics|D2Metrics|D3Metrics crates/` returns zero hits). Today D1 runs per-episode via `spawn_episode_distillation` (see D.09), with no batching, no budget, and no quality floor. The only cadence signal in the codebase is `roko-learn::runtime_feedback::UpdateFrequency::distiller_every_n: u32` (default 50, `runtime_feedback.rs:175, 213`) plus its `distiller_due` helper at `:200-202`; however the check at `runtime_feedback.rs:862` drives `self.append_cfactor_snapshot()` — a C-factor (efficiency) metric, not a distillation call. No `.roko/learn/distillation-quality.jsonl` file is written (`rg distillation-quality crates/` returns zero hits). D2 currently runs inline as part of `TierProgression::analyze` on every Dream cycle (D.10) with no interval gating and no min_new_insights check. D3 (`write_playbook`) is not called at all (see D.03).
**Fix sketch**: Either remove these two PRD sections (they describe a system that could be built but is not present in any form) or build the scheduler against the existing `UpdateFrequency` infrastructure — rename `distiller_every_n` to something specific, wire a real `should_run_d1` check into the episode completion hook at `orchestrate.rs:693-698`, and emit `DistillationQualityReport` from `DreamCycle::run_budgeted` after the `analyze + replay_heuristics` block at `cycle.rs:428-430`. The quality-metric math (survival rate, novelty rate, contradiction rate) has no underlying telemetry today — adding it would require tracking D1-output → promotion lifetime, which the current `KnowledgeStore` does not instrument.

---

## Section Summary

| Status | Count |
|--------|-------|
| DONE | 3 (D.04 Distiller struct, D.09 spawn hook, D.10 Dreams wiring) |
| PARTIAL | 7 (D.01 D1 dual-path, D.02 missing cross-validation, D.03 playbook path + format, D.05 trait signature, D.06 TierProgression fields, D.07 variant names, D.08 data-type drift) |
| NOT DONE | 4 (D.11 extract_warnings, D.12 HdcClusterer, D.13 promotion gates, D.14 scheduler + quality report) |
| SCAFFOLD | 0 |

The core pipeline is real and wired: `Distiller` uses Claude Haiku, `spawn_episode_distillation`
fires from five call sites, and `DreamCycle::run_budgeted` calls `TierProgression::analyze +
replay_heuristics` on every cycle (`cycle.rs:428-430`). The `5+ @ ≥0.7` core promotion threshold
is faithfully encoded in `DEFAULT_MIN_HEURISTIC_SUPPORT = 5` and `DEFAULT_MIN_CONFIDENCE = 0.7`
(`tier_progression.rs:25-26`). The tier feedback loop through `TierProgressionDecision` is wired
into the orchestrator at `orchestrate.rs:12912-12956`.

The drift is almost entirely at the struct-shape / field-name level. The `DistillationBackend`
trait has `complete(prompt) + model()` not `distill(episode)`; `TierProgression` holds thresholds
not `Arc<Mutex<KnowledgeStore>>`; `TierProgressionDecision` variants are
`Promote/Demote/ReviewExpiry/NoChange` not `Promote/Demote/Retain/Retire`; `InsightRecord` uses
`antecedent+consequent` not `pattern`; `HeuristicRule` has 12 fields (including `source_model`
and `model_generality`) not 4; `PlaybookCompilation` has no `title` field; `write_playbook` is
parametric and uncalled, so `.roko/neuro/PLAYBOOK.md` is the doc's claimed path, not a real file
the system writes today (the actual canonical playbook path is `.roko/memory/playbook.toml` in
`roko-fs/src/layout.rs:221-223`, an entirely different artefact belonging to the dreams crate).

The large designed-but-unimplemented blocks (lines 285-763 of the doc: `extract_warnings`,
`HdcClusterer`, `cross_validation_check`, `anti_knowledge_check`, `DreamsDistillationTrigger`,
`DistillationQualityReport`, `DistillationScheduler`) are correctly self-annotated as "Missing"
at lines 775-781 — each is now broken out as D.11–D.14 so the reality column has a concrete
grep-verified anchor per PRD §. The `DreamsDistillationTrigger` API itself is effectively
covered by D.10 (the existing `DreamCycle::run_budgeted` is the real trigger), so no new item
is required for that designed type.

## Agent Execution Notes

### D.01-D.03 / D.11-D.13 — Harden The Real Pipeline First

The right question here is not “how do we implement every normative Rust block from doc 12?”

It is:

1. what deterministic checks belong on the current D1/D2/D3 path now,
2. what is the real playbook output contract,
3. which designed-only helpers should be explicitly demoted instead of implied.

### D.14 — Keep Scheduling Honest

If an agent touches scheduling or quality reporting:

- prefer one minimal cadence/report surface tied to existing hooks,
- or explicitly map `spawn_episode_distillation` plus Dream-cycle analysis and stop.

Do not build a large distillation operations layer without first proving a concrete caller.

Acceptance criteria for this section:

- later agents can answer when distillation runs,
- promotion guards are either real or clearly deferred,
- the playbook output path is no longer ambiguous.
