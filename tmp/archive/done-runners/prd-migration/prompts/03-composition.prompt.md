# Prompt: 03-composition

You are a fresh Claude Opus agent. Zero prior context. Read every file this prompt references before writing.

## Your mission

Generate `/Users/will/dev/nunchi/roko/roko/docs/03-composition/`. This topic covers the
Scaffold layer (L2): prompt assembly, system prompt builder, enrichment pipelines, token
budget management, Lost-in-the-Middle U-shape, active inference for context selection,
5-stage assembly pipeline, predictive foraging (MVT stopping), VCG attention auction,
distributed context engineering.

## Step 1 — Context pack (MANDATORY, in order)

Read all 7 files in `/Users/will/dev/nunchi/roko/roko/tmp/prd-migration/context-pack/` in order (00 through 06).

## Step 2 — refactoring-prd canonical sources

1. `/Users/will/dev/nunchi/roko/refactoring-prd/02-five-layers.md` — §Layer 2 Scaffold, §Context as Active Inference, §Predictive Foraging, §Three Levels of Context Engineering
2. `/Users/will/dev/nunchi/roko/refactoring-prd/09-innovations.md` — §II VCG Attention Auction, §XIX.B EFE for Context Selection, §XIX.C Context Foraging Stopping Rule (MVT), §XIX.E VCG Bid Computation, §XV Distributed Context Engineering
3. `/Users/will/dev/nunchi/roko/refactoring-prd/01-synapse-architecture.md` — §Composer trait signature (sync, takes &dyn Scorer, Budget struct)
4. `/Users/will/dev/nunchi/roko/refactoring-prd/08-translation-guide.md`
5. `/Users/will/dev/nunchi/roko/refactoring-prd/07-implementation-priorities.md` §Tier 2C (Context assembly with active inference)

## Step 3 — SOURCE-INDEX entry `## 03-composition.md`

Read every file listed. Key legacy sources:
- `bardo-backup/prd/12-inference/04-context-engineering.md`
- `bardo-backup/prd/12-inference/02-caching.md`, `05-sessions.md`, `06-memory.md`, `13-reasoning.md`, `14-rust-implementation.md`, `15-inference-profiles.md`, `16-structured-outputs.md`, `19-multi-model-orchestration.md`
- `bardo-backup/prd/25-mori/mori-context-engineering.md`, `mori-context-service.md`, `mori-cost-efficiency.md`
- `bardo-backup/tmp/mori-refactor/05-scaffold.md` — full scaffold layer spec
- `bardo-backup/tmp/mori-refactor/08-inference-optimization.md`, `22-cost-optimization-architecture.md`
- `bardo-backup/tmp/mori-agents/04-context-engineering.md`, `05-prompt-engineering.md`, `17-dynamic-prompt-generation.md`, `24-prompt-budget-engineering.md`, `mori-context-optimization.md`
- `bardo-backup/tmp/agent-chain/15-dynamic-context-assembly.md`, `context-quality-science.md`, `harness-engineering.md`
- `bardo-backup/tmp/mori-refactor-plan/12-context-data-optimization.md`, `14-optimization-playbook.md`

## Step 4 — implementation-plans

- `12a-cognitive-layer.md` §E (E1–E6: 5-stage pipeline, active inference scoring, attention-curve U-shape, affect-modulated retrieval, dynamic budget, neuro injection)
- `modelrouting/13-architectural-gaps.md` §B Cache Layers

## Step 5 — active code

- Glob `/Users/will/dev/nunchi/roko/roko/crates/roko-compose/src/**/*.rs`
- Read: `lib.rs`, `system_prompt_builder.rs`, `context_provider.rs`, `enrichment/` (all files), `scorer.rs`, `role_prompts.rs`, `composer.rs` (if exists)

## Step 6 — output dir and sub-docs

```bash
mkdir -p /Users/will/dev/nunchi/roko/roko/docs/03-composition
```

Write **14 sub-docs** plus `INDEX.md`:

| # | Filename | Content |
|---|---|---|
| 00 | `00-composer-trait.md` | Composer trait (sync, takes &dyn Scorer). Budget struct (max_tokens, max_signals, max_bytes). Why composer takes a Scorer parameter (rank inputs under budget). Rust signature. |
| 01 | `01-prompt-composer.md` | PromptComposer implementation. Priority-based dropping. Section packing. U-shape placement per Liu et al. 2023 "Lost in the Middle" (arXiv:2307.03172). |
| 02 | `02-system-prompt-builder-6-layer.md` | The 6-layer SystemPromptBuilder. Role-aware templates. Layer composition. How the layers build a complete system prompt. |
| 03 | `03-role-templates.md` | 9 role templates. Implementer, Reviewer, Scribe, Architect, Researcher, etc. Per-role prompt content. Template source: `roko-compose/src/templates/`. |
| 04 | `04-enrichment-pipeline-13-step.md` | The 13-step enrichment pipeline with batch/direct LLM clients. Each step explained. Pre-computed context artifacts (briefs, decompositions, research memos). |
| 05 | `05-token-budget-management.md` | budget_for(role) function. Stanza constants. Priority-based dropping under budget. Dynamic token allocation across sections. Budget enforcement. |
| 06 | `06-lost-in-the-middle-u-shape.md` | Liu et al. 2023 "Lost in the Middle" (arXiv:2307.03172). U-shaped attention curve. Implementation: place highest-value sections at start and end. Middle positions as secondary. |
| 07 | `07-active-inference-context-selection.md` | EFE (Expected Free Energy) for context section selection. Full formula from 09-innovations.md §XIX.B: `G(section) = pragmatic_value + epistemic_value - ambiguity`. pragmatic_value = cosine similarity with task goal. epistemic_value = 1 - max similarity to already selected. Ambiguity = entropy estimate. Selection: softmax(gamma × G) with gamma=8.0. Parr et al. 2024 (arXiv:2402.14460). Koudahl et al. 2024 (arXiv:2412.10425). |
| 08 | `08-5-stage-assembly-pipeline.md` | Query → Score → Deduplicate → Budget → Format. From implementation-plans/12a §E1. Active inference scoring formula: `score = track_record(entry) × belief_change(entry) / uncertainty`. Each stage in detail. |
| 09 | `09-predictive-foraging-mvt.md` | Patch-leaving via Marginal Value Theorem (Charnov 1976). Formula: stop when relevance(last) / cost ≤ total_gain / total_cost. Exponential gain curve g(k) = G_max × (1 - exp(-λk)). Fit λ online from first retrievals. Hard floor: relevance < 0.05. Scent following (Pirolli & Card 1999). Exploration budget before implementation. |
| 10 | `10-vcg-attention-auction.md` | Vickrey-Clarke-Groves auction (Vickrey 1961, Clarke 1971, Groves 1973). Truthful bidding for limited context budget. Why VCG not priority ranking. Full bid formula from 09-innovations.md §XIX.E: `bid(section) = expected_value × urgency × affect_weight`. Winner pays second price. Bidding subsystems (Neuro, Daimon, iteration memory, code intelligence, playbook rules, research artifacts, task context, oracle predictions). Affect modulation. |
| 11 | `11-distributed-context-engineering.md` | Context engineering at network scale (not just single agent). Four strategies: Write (persist), Select (retrieve), Compress (HDC bundling), Isolate (sandboxed sub-contexts). Karpathy 2025 on context engineering. Meta-Harness (Lee et al. 2026, arXiv:2603.28052) with +7.7/+4.7 result nuance. Table comparing prompt engineering vs context engineering vs distributed CE. |
| 12 | `12-affect-modulated-retrieval.md` | PAD state biases which knowledge is surfaced. High arousal → recent + action-oriented. Low pleasure → include past failure context. Integration with Daimon. Cross-reference topic 09-daimon. |
| 13 | `13-current-status-and-gaps.md` | roko-compose has 23 tests. SystemPromptBuilder wired into orchestrate.rs via RoleSystemPromptSpec. What's built vs. scaffold. 5-stage pipeline not yet wired (implementation-plans/12a §E1). Active inference scoring is target (E2). U-shape positioning is target (E3). |

Plus `INDEX.md`.

## Step 7-9 — Rules + INDEX + self-check

Per context-pack rules. Minimum 200 lines per sub-doc. Total ≥3500 lines. Citations preserved (ACE, CSO, ACON, RAGAS, ARES, FrugalGPT, Meta-Harness, Karpathy, Liu 2023, Charnov 1976, Pirolli & Card 1999, Vickrey, Clarke, Groves, Parr 2024, Koudahl 2024).

Cross-reference topics: 00-architecture, 02-agents (which calls the Composer), 06-neuro (knowledge injected into context), 09-daimon (affect-modulated retrieval), 16-heartbeat (3-speed cognition).

## CRITICAL REMINDERS

- DO NOT SUMMARIZE. DO NOT TRUNCATE. PRESERVE ALL CITATIONS.
- Apply naming map: golem → agent; bardo-gateway → roko gateway; mori → Roko Orchestrator.
- The 5-stage pipeline is: Query → Score → Deduplicate → Budget → Format. Active inference scoring formula: `score = track_record × belief_change / uncertainty`.
- VCG auction: subsystems pay second-highest bid (truthfulness). Payment deducted from subsystem's attention budget for next tick.
- MVT stopping rule for context foraging — full algorithm per 09-innovations.md §XIX.C.
- Use Write tool. Absolute paths. Don't ask questions.
