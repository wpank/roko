# roko-compose — Prompt Assembly & Context Engineering
> Status-quo audit · verified 2026-07-08 @ HEAD 5852c93c05 · sources: 52 crate files (~26.8K LOC) read/grepped, orchestrate.rs call sites traced, 15 v1 + 6 v2 design docs, `.roko/GAPS.md`, roko-graph cells
> Re-verification 2026-07-08: all headline claims re-checked against current code. VCG-unreachable, empty-`learning_bidders`, zero external `update_bidders` callers, `ActiveInferenceScorer` alias, and no `ComposeProtocol`/`ComposeBid`/`ComposeResult` types anywhere — **all still hold**. Line numbers within ±0–5. Newly surfaced: a fourth dormant assembly surface (`templates/assembly.rs` `PromptAssembler`) — see tech-debt.
>
> **DEEP SECOND PASS 2026-07-08 @ HEAD 5852c93c05** (see bottom sections): (1) full per-layer/per-slot content+source table; (2) **the live default `roko plan run` is Runner v2, not orchestrate.rs, and it BYPASSES the canonical Compose stack** — `PlanEngine::RunnerV2` is `#[default]` (main.rs:1301) and its prompt is built by a self-contained CLI-side `PromptAssembler` (`dispatch/prompt_builder.rs:717`) that never calls `SystemPromptBuilder`; (3) SIX assembly surfaces catalogued (was 4); (4) VCG warmup threshold = `DEFAULT_VCG_WARMUP_OBSERVATIONS = 10` (strategy.rs:10); (5) 11 role templates + 13 enrichment prompt constants enumerated.

## Summary

roko-compose is a large, well-tested **v1-shape library** (52 files, ~26.8K LOC, ~430 `#[test]` markers + 2 integration suites) called synchronously from `orchestrate.rs::dispatch_agent_with` (orchestrate.rs:15184). The core assembly path is genuinely wired: 12-slot **SystemPromptBuilder** ("9-layer" is the official count — the v1 doc title "7-layer" is stale; its own body says 9), 13/13 **enrichment steps** executing per-plan via `run_enrichment_pipeline`, tiered **ContextProvider**, real tokenizer-backed budgets, U-shaped placement, affect-modulated retrieval via roko-neuro, and a **section-effectiveness learning loop** that persists to `.roko/learn/section-effects.json`.

The headline gap: **VCG is unreachable at runtime**, not merely "dominated". `CompositionStrategy::Auto` only flips to VCG when every active bidder has ≥10 learned observations (strategy.rs:10,48-58), but orchestrate constructs `PromptComposer::new()` with an empty `learning_bidders` map (orchestrate.rs:16377, prompt.rs:633-637) and **nothing anywhere calls `update_bidders`/`update_with_cost`** — observations are permanently 0, so density-greedy runs 100% of the time. CLAUDE.md's "Partial" is confirmed and understated. Similarly built-but-dormant: MVT foraging pre-pass, HDC dedup (feature not even enabled by roko-cli), dynamic placement, BudgetPredictor, ContextMesh, and the true-EFE scorer (shipped as a documented heuristic alias). There is no v2 `ComposeProtocol`/`ComposeBid` anywhere, and the roko-graph `system-prompt-builder` cell is a `PassthroughCell` stub (roko-graph/src/cells/stubs.rs:72).

## Current state table

| Component | Design source | Code | Status | Evidence |
|---|---|---|---|---|
| Compose verb trait (v1) | v1/03/00 | `impl Compose for PromptComposer` / `SystemPromptBuilder` | ✅ wired (🕰️ shape) | prompt.rs:780; system_prompt_builder.rs:878 |
| PromptComposer (budget knapsack) | v1/03/01 | `compose()`: decode→critical split→bid density→strategy→placement sort | ✅ wired | prompt.rs:781-960; called orchestrate.rs:16418-16425 |
| SystemPromptBuilder layers | v1/03/02 ("7-layer", body says 9) | 12 section slots: 1 role, 2 conventions, 3 domain, 3b context, 3c pheromones, 4 task, 4b gate_feedback, 5 tools, 6 skills+playbooks, 6b tool_hints, 7 anti_patterns, 8 affect | ✅ wired — **9-layer claim correct, 7 is stale; 12 concrete slots** | system_prompt_builder.rs:1,50,62-98,466-642; `layer_count()` :646-679 counts ≤11 (misses tool_hints) |
| Role templates | v1/03/03 | 10 modules / 11 template structs (Conductor, Implementer, Integration, QuickFix, QuickReviewer, Refactorer, Researcher, Reviewer, Scribe+variants, Strategist, TaskImpl) + `RolePromptTemplate` trait | ✅ wired | templates/mod.rs:9-34,82-108; ReviewerTemplate orchestrate.rs:18741; role identities via role_prompts.rs:717 |
| RoleSystemPromptSpec | CLAUDE.md | Spec → SystemPromptBuilder → build/build_with_counter | ✅ wired | role_prompts.rs:230-261,405,451; `roko-cli/src/prompting.rs:38-96` (builder path, uses `composition_scorer` → `GoalDirectedHeuristicScorer`, confirming COMP-05 alias at role_prompts.rs:691-697); callers: orchestrate.rs:20545, run.rs:1517, dispatch_helpers.rs:133, prompt_helpers.rs:94 |
| Template assembly path (`PromptAssembler`) | roko-core foundation | `templates/assembly.rs`: manual U-shape sort + budget knapsack over `RolePromptTemplate::sections`, `.assemble`/`.assemble_from`; **no roko-cli caller** — a 4th assembly surface distinct from `PromptComposer` | 🔌 built-not-wired | templates/assembly.rs:150-204; grep `TemplateAssembly`/`assemble_from` in roko-cli/src → 0 hits |
| Enrichment pipeline (13 steps) | v1/03/04 | All 13 in `EnrichStep`: Prd, Briefs, Tasks, Decompose, Research, Dependencies, Fixtures, Integration, Verify, Reviews, Tests, Invariants, Scribe | ✅ wired 13/13 | enrichment/step.rs:53-98; `run_steps` called orchestrate.rs:9416-9445 from `handle_enriching` :9485; complexity StepSelector :2011 |
| Enrichment adaptive skip (COMP-09) | v1/03/04 | `StepOutcomeHistory` + `with_outcome_history` exist; orchestrate never attaches history | 🔌 built-not-wired | pipeline.rs:39-81,277; absent in orchestrate.rs:9416-9429 |
| Token budget mgmt | v1/03/05 | `TokenCounter` (tiktoken o200k / HF tokenizers / heuristic), per-role `PromptBudget`, `Complexity` scaling, `build_with_counter` enforcement, `Budget::tokens(config.prompt.token_budget)` | ✅ wired | token_counter.rs:9-63; budget.rs:23-82; system_prompt_builder.rs:373-457; orchestrate.rs:16421; config.rs:467-482 |
| BudgetPredictor (learned budget) | v1/03/05 | Built + `load_predictor(.roko/learn)`; zero callers outside crate | 🔌 built-not-wired | budget_predictor.rs:95,410; no non-compose refs |
| Lost-in-the-middle (static U-shape) | v1/03/06 | `Placement` Start/Middle/End per layer; final sort by placement | ✅ wired | prompt.rs:905-910; layer placements system_prompt_builder.rs:484-635 |
| PositionAttentionModel / `dynamic_placement` | v1/03/06, v2 positional-effects | Fitted-curve model, per-model curves, query-aware reassignment — exported, uncalled | 🔌 built-not-wired | attention.rs:15-111; no callers outside crate |
| Active-inference selection | v1/03/07, v2 active-inference | `ActiveInferenceScorer` = **alias** for `GoalDirectedHeuristicScorer` (HDC hash embeddings, not Bayesian EFE — documented COMP-05 tradeoff); runtime uses `SectionScorer`+`CatalystScorer`+`PredictiveScorer` sum instead | 🟡 partial | scorer.rs:260-295; orchestrate.rs:16382-16395 |
| 5-stage assembly | v1/03/08 | Score→(Dedup)→Budget→Format in composer; Query/retrieve upstream in ContextProvider + neuro ContextAssembler | 🟡 partial (dedup off) | prompt.rs:828-910; orchestrate.rs:15991,16072 |
| Foraging MVT (COMP-03) | v1/03/09 | `MultiPatchForager` + `foraging_prepass` in composer; `with_foraging` never called → always `None` | 🔌 built-not-wired | prompt.rs:613,754-760,857-861,1595-1627; foraging.rs:25 |
| HDC dedup (COMP-04) | v1/03/08 | Behind `hdc` feature; default features empty; roko-cli doesn't enable; threshold defaults 0.0, `with_hdc_dedup` uncalled | ❌ missing at runtime | Cargo.toml:29-31; roko-cli/Cargo.toml:35; prompt.rs:637,774,852-855 |
| VCG attention auction | v1/03/10, v2 vcg-attention-auction | `vcg_allocate`, payments, displacement diagnostics, affect modulation, fairness, Pareto check all built; **runtime always resolves DensityGreedy** (empty bidders → 0 obs < 10 warmup); VCG exercised only in unit tests | 🔌 built-not-wired (CLAUDE.md "partial" confirmed, stronger) | strategy.rs:10-58; prompt.rs:633-637,863-897,1189-1252; auction.rs:688 LOC; orchestrate.rs:16377; zero `update_bidders`/`with_learning_bidders`/`with_strategy` callers |
| Greedy path + pseudo-diagnostics | v1/03/10 | Density sort + diversity boost 1.18 / diminishing 0.82^wins / affect multiplier; VCG payment summary still computed for diagnostics | ✅ wired | prompt.rs:874-896,1254-1264 |
| AttentionBidder variants | CLAUDE.md "Neuro/Task/Research wired" | 8 variants; runtime-attached: TaskContext, Oracles, Research, Neuro (orchestrate), Daimon (dispatch/prompt helpers), IterationMemory (builder 4b). CodeIntelligence + PlaybookRules only in tests | 🟡 partial (6/8) — CLAUDE.md claim confirmed | prompt.rs:83-101; orchestrate.rs:474,16167,16216,16240,16286; dispatch_helpers.rs:257; system_prompt_builder.rs:582 |
| CompositionManifest | v2 compose-protocol | Emitted as signal tag w/ strategy, included/excluded+reasons, payments; consumed only for `last_prompt_sections` (TUI) | 🟡 partial | prompt.rs:938-949,1435-1540; orchestrate.rs:16426-16430 |
| Tiered ContextProvider | v1/03/11 | 3.5K LOC; tier/budget resolve wired w/ config overrides, attribution keys extracted | ✅ wired | context_provider.rs; orchestrate.rs:15991-16051; config.rs:480-515 |
| Distributed context (ContextMesh, Level 3) | v1/03/11, v2 distributed | Registry built; no callers outside crate | 🔌 built-not-wired | context_mesh.rs:50; lib.rs:65 |
| Affect-modulated retrieval (COMP-06) | v1/03/12, v2 affect | Canonical assembler lives in roko-neuro, re-exported; `with_affect_state` + PAD-modulated auction via `roko.daimon.*` ctx attrs + builder Layer 8 guidance | ✅ wired | context_assembler.rs:1-4; orchestrate.rs:16402-16416,19511-19545; system_prompt_builder.rs:626-720 |
| Section-effectiveness learning | v2 (closest: process-reward) | Registry snapshot → priority overrides in builder + orchestrate sections; updated from efficiency events → `.roko/learn/section-effects.json` | ✅ wired | orchestrate.rs:16133,16281; system_prompt_builder.rs:98,482; roko-learn/runtime_feedback.rs:1711,2817-2834 |
| Cache alignment (4 tiers) | v1/03/02 | `CacheLayer` Role/Workspace/Plan/Volatile + `<!-- cache:* -->` markers + `normalize_for_caching`; snapshot + stability tests | ✅ wired | system_prompt_builder.rs:107-122,1400-1406; tests/cache_stability.rs; tests/system_prompt_snapshot.rs |
| PromptAssemblyService (`PromptAssembler` trait impl) | roko-core foundation | Used by roko-orchestrator `service_factory` + roko-runtime `workflow_engine` (Runner-v2/engine path), not by orchestrate.rs | 🟡 partial | prompt_assembly_service.rs:1150 LOC; roko-orchestrator/service_factory.rs:210,233; roko-runtime/workflow_engine.rs:1323 |
| Compose as v2 Cell/protocol | v2 compose-protocol-and-builder | No `ComposeProtocol`/`ComposeBid`/`ComposeResult` types anywhere; graph cell is stub | ❌ missing | grep: zero hits; roko-graph/src/cells/stubs.rs:72, engine.rs:366; GAPS.md:16 |

## V2-aligned
- **13-step enrichment pipeline as ordered chain** with per-step typed artifacts, staleness/freshness checks, cost tracking — close to v2's "Pipeline Graph of Cells" minus Cell packaging (enrichment/pipeline.rs; enrichment/step.rs:108-120 artifact filenames).
- **Bid-shaped internals**: `AuctionCandidate{bid_value, bid_density}`, `VcgBid`, manifests with per-section inclusion reasons — the data model v2's `ComposeBid` wants already exists (prompt.rs:1030, auction.rs:20-55).
- **Section-effectiveness feedback loop** (include→outcome→priority adjust) is a working instance of v2 process-reward thinking (orchestrate.rs:16133; runtime_feedback.rs:2817).
- **Affect endofunctor precedent**: PAD state flows through ctx attrs and modulates both retrieval and auction, matching v2 distributed-and-affect-composition direction (orchestrate.rs:16402-16416; prompt.rs:1267+).

## Old paradigm & tech debt
- 🕰️ **Library, not protocol**: compose is invoked inline from the 23.7K-line `dispatch_agent_with`; scorer passed as `&dyn Score` exactly as v2 says to replace (prompt.rs:781-787).
- 🕰️ **Four+ parallel prompt-assembly surfaces**: (1) orchestrate.rs sections+`PromptComposer` (the wired runtime path, orchestrate.rs:16377-16425); (2) `RoleSystemPromptSpec::build` → `PromptComposer` via `role_prompts.rs:655` (spec path, used by prompting.rs/run.rs/dispatch_helpers); (3) `dispatch/prompt_builder.rs` (Runner v2 wrapper over the 9-layer builder, prompt_builder.rs:7,714) and `chat_session.rs:1521` direct builder; (4) **`templates/assembly.rs` `PromptAssembler`** — a self-contained knapsack+U-shape assembler over role templates, **zero callers** (newly catalogued 2026-07-08). roko-serve `dispatch.rs:1999` adds a degenerate 1-layer `SystemPromptBuilder::new(prompt).build()`. Consolidation target still open (see Open Q2).
- **Duplicate `GateFeedback` types** in roko-compose (gate_feedback.rs) and roko-gate (feedback.rs:53) plus `DispatchGateFeedback` in runner (event_loop.rs:4246) — three shapes for one concept.
- **Doc drift**: v1 doc filename says "7-layer", code says 9, concrete slots are 12, and `layer_count()` can't count past 11 (misses `tool_hints`) — system_prompt_builder.rs:646-679.
- **Dead-weight exports**: `is_pareto_optimal`, `detect_bid_correlation`, `FairnessConfig`, `LearningBidder`, `BudgetPredictor`, `ContextMesh`, `dynamic_placement`, `compact_history`, `CostAttribution`, `build_cognitive_workspace`, `PromptAssembler` (templates/assembly.rs) — public API with no runtime consumers.
- **No config surface for composition strategy** — `[prompt]` has only `token_budget`/`role`/`files`/`context_budgets` (config.rs:467-482,1740-1755); strategy/warmup/foraging/dedup are unreachable without code changes.

## Not implemented
- **v2 `ComposeProtocol` trait + `ComposeBid` inputs + Cell packaging** — nothing exists; roko-graph `system-prompt-builder` cognitive-loop cell is a `PassthroughCell` stub (GAPS.md:16).
- **LearningBidder observation loop** — no persistence, no post-gate `update_bidders`/`update_with_cost` call, so VCG warmup can never complete.
- **True EFE scoring** — KL-divergence epistemic value acknowledged as unbuilt; heuristic alias shipped instead (scorer.rs:260-295).
- **Runtime HDC dedup** — feature flag never enabled by any binary crate.
- **Learned layer ordering / DSPy-style prompt optimization** — listed in v1/03/13 gaps; no code found.
- **Per-model fitted attention curves at runtime** — `ModelAttentionCurves::save` exists, nothing fits or loads curves.

## Migration checklist
- [ ] **[P0]** Close the VCG warmup loop: persist `LearningBidder`s (e.g. `.roko/learn/attention-bidders.json`), call `composer.update_bidders(included, gate_passed)` after each gate verdict in `dispatch_agent_with`/`finish_task_post_processing`, and construct the composer via `with_learning_bidders` — verify: `roko plan run plans/ && grep -c '"selected_strategy":"vcg"' .roko/signals.jsonl` (expect >0 after ~10 tasks)
- [ ] **[P0]** Add `[prompt] composition_strategy` + `vcg_warmup_observations` config knobs feeding `PromptComposer::with_strategy` — verify: `cargo run -p roko-cli -- config show | grep composition_strategy`
- [ ] **[P1]** Define v2 `ComposeProtocol` (bids in, `ComposeResult` w/ payments out) in roko-core and adapt `PromptComposer` behind it; replace the roko-graph `system-prompt-builder` PassthroughCell with a real cell delegating to `RoleSystemPromptSpec::build_sections` — verify: `grep -rn 'ComposeProtocol' crates/roko-core crates/roko-graph --include='*.rs' | grep -v test`
- [ ] **[P1]** Attach `StepOutcomeHistory` (load/persist under `.roko/learn/enrichment-history.json`) in `run_enrichment_pipeline` so adaptive skip fires — verify: `grep -n 'with_outcome_history' crates/roko-cli/src/orchestrate.rs`
- [ ] **[P1]** Tag code-intelligence and playbook sections with `AttentionBidder::CodeIntelligence`/`PlaybookRules` at dispatch (currently default TaskContext) — verify: `grep -n 'AttentionBidder::CodeIntelligence' crates/roko-cli/src/orchestrate.rs`
- [ ] **[P2]** Enable `hdc` feature in roko-cli + set `with_hdc_dedup(0.15)` per v1/03/08 dedup stage — verify: `cargo tree -p roko-cli -e features | grep 'roko-compose.*hdc'`
- [ ] **[P2]** Wire `with_foraging(MultiPatchForager)` from a config/learned profile so MVT pre-pass runs — verify: `grep -rn 'with_foraging' crates/roko-cli/src`
- [ ] **[P2]** Unify the three GateFeedback types (compose/gate/runner) into one roko-gate type consumed by compose — verify: `grep -rn 'struct.*GateFeedback' crates/ --include='*.rs' | grep -v test | wc -l` (expect 1)
- [ ] **[P3]** Wire `dynamic_placement` + `ModelAttentionCurves` load into composer format stage; fit curves from episode data — verify: `grep -rn 'dynamic_placement' crates/roko-cli/src`
- [ ] **[P3]** Retire or wire dormant exports (`BudgetPredictor`, `ContextMesh`, `CostAttribution`, `compact_history`) and fix `layer_count()` to include `tool_hints` — verify: `cargo clippy -p roko-compose --no-deps -- -D warnings` after adding `#[deprecated]`/call sites
- [ ] **[P3]** Rename v1 doc `02-system-prompt-builder-7-layer.md` or add erratum; align CLAUDE.md wording ("9-layer, 12 slots") — verify: `grep -rn '7-layer' docs/v1/03-composition/`

## Open questions
1. Should VCG become default-on once warm, or stay diagnostics-only? v2 doc mandates VCG; greedy+diversity-boost already approximates its welfare goal — need an A/B via `ExperimentStore` before flipping.
2. Which of the two runtime assembly paths wins long-term — orchestrate's inline sections+`PromptComposer`, or `PromptAssemblyService` behind `roko_core::foundation::PromptAssembler` (Runner v2 / workflow engine)? Consolidation target unclear.
3. Is `ActiveInferenceScorer`'s heuristic alias acceptable permanently (COMP-05 rationale is well-argued), or does v2 active-inference doc require real EFE for the protocol version?
4. ContextMesh (Level 3 distributed context) has no runtime multi-agent consumer — does the mesh belong in roko-compose at all, or in roko-serve/agent-server where agents actually coexist?
5. `layer_count()` says ≤11, docs say 9, slots are 12 — what is the canonical layer taxonomy for v2's `ComposeBudget` per-layer accounting?

---

# DEEP SECOND PASS — concrete layer/slot content, live-path trace, surface census, VCG path, templates
> Verified 2026-07-08 @ HEAD 5852c93c05. Every row below carries file:line. Status tags: ✅ wired · 🟡 partial · 🔌 built-not-wired · ❌ missing.

## 1. The 9 layers / 12 concrete slots — exact content + source

The `SystemPromptBuilder` doc header advertises "9 layers" (system_prompt_builder.rs:1,10-26). The struct declares **12 emittable slots** (fields at system_prompt_builder.rs:62-99). `build_sections()` (system_prompt_builder.rs:466-642) emits them; `SECTION_SPECS` (system_prompt_builder.rs:1258-1319) is the single-source-of-truth registry that drives heading text, order-rank, and budget-cap lookups. Emission order in code is **not** the final rendered order — `sort_sections` (system_prompt_builder.rs:1089-1097) re-sorts by `cache_layer` → `priority` DESC → `order_rank`.

| # | Slot (`PromptSection` name) | Doc layer | Content it contributes | Priority | Cache tier | Placement | Source (builder emit) | Content origin |
|---|---|---|---|---|---|---|---|---|
| 1 | `role_identity` | L1 Role identity | "You are the **{role}**…" identity paragraph (+ optional temperament stanza) | **Critical** | Role | Start | sysprompt:469-487 | `role_identity_for(role)` role_prompts.rs:717-744 → per-role `Template.role_identity()` |
| 2 | `conventions` | L2 Conventions | Project coding standards (snake_case, thiserror, no-unwrap…) | High | Role | Start | sysprompt:490-503 | `RoleSystemPromptSpec::conventions_text()` role_prompts.rs |
| 3 | `tool_instructions` | L5 Tools | MCP tool allowlist stanza + "use only granted tools" | Normal | Role | Middle | sysprompt:506-519 | `tool_allowlist_instructions(csv)` role_prompts.rs:702-712 |
| 4 | `domain_context` | L3 Domain | Project/domain knowledge block | High | Workspace | Middle | sysprompt:522-535 | `task_context.domain_layer()` role_prompts.rs:432 |
| 5 | `context_layer` | L3b Context | "## Relevant Context\n…" assembled retrieval | High | Workspace | Middle | sysprompt:538-551 | `task_context.context_layer()` role_prompts.rs:436; upstream ContextProvider/neuro |
| 6 | `pheromone_signals` | L3c Active signals | "## Active Signals" — sorted ContextChunks, labelled Threat/Warning/Opportunity/Signal, recency/confidence/track_record; hard_cap 1500 | High | Workspace | Middle | sysprompt:553-556, `pheromone_section` 1330-1358 | `with_pheromones` ← stigmergic chunks |
| 7 | `task_context` | L4 Task | Current task details | **Critical** | Plan | End | sysprompt:558-572 | `task_context.task_layer()` role_prompts.rs:439 |
| 8 | `gate_feedback` | L4b Gate feedback | "## Gate Feedback" retry digest; bidder `IterationMemory`; hard_cap 1500 | High | Volatile | End | sysprompt:574-587 | `GateFeedback::render_prompt_section()` / `from_raw` (gate_feedback.rs) |
| 9 | `relevant_techniques` | L6 Techniques | "## Relevant Techniques" — ≤3 playbooks + skills, greedily budget-trimmed; bidder `PlaybookRules`; hard_cap = `budget.skills/4` | High | Plan | End | sysprompt:589-592, `relevant_techniques_section` 769-842 | `with_playbooks`/`with_skills` ← roko-learn |
| 10 | `tool_hints` | L6b Tool hints (LEARN-12) | Learned tool-sequence hints | **Low** | Plan | Middle | sysprompt:594-604 | `with_tool_hints` ← learned profiles |
| 11 | `anti_patterns` | L7 Anti-patterns | "Do NOT:\n- …" bulleted list | Normal | Plan | End | sysprompt:606-624 | `RoleSystemPromptSpec::anti_patterns()` role_prompts.rs:408 |
| 12 | `affect_guidance` | L8 Affect | PAD-derived tone/focus prose (arousal/pleasure/dominance/somatic branches) | Normal | Volatile | End | sysprompt:626-638, `affect_guidance` 681-743 | `with_affect_state(PadState)` ← daimon `roko.daimon.*` ctx attrs |

Notes:
- **`layer_count()` is buggy** (sysprompt:646-679): counts a max of 11 and never counts `tool_hints` — so it disagrees with both the "9" doc claim and the 12 real slots.
- **Priority defaults** (`SectionPriority`) set drop-order under budget: only `role_identity` + `task_context` are Critical (never dropped; truncated-to-fit if needed, sysprompt:430-448); `tool_hints` is Low (first to go).
- **Temperament** (AGT-06) is folded into slot 1, not a separate slot — `temperament_guidance()` sysprompt:1065-1087 (Conservative/Aggressive/Exploratory prose; Balanced = "").
- **Cache markers** (opt-in `with_cache_markers`) insert `<!-- cache:system|session|task|dynamic -->` between tiers (`cache_marker` sysprompt:1400-1407).
- Learned **section-effectiveness** can bump/drop each slot's priority by ±1 (`effective_priority` sysprompt:745-755 → `adjusted_priority` 900-911) and reweight budget caps (`apply_learned_budget_tuning` 931-996).

## 2. Live-path prompt-assembly trace — the default runner BYPASSES the canonical builder

**Command:** `roko plan run <dir>` → `PlanCmd::Run` handler plan.rs:230.
**Engine selection:** `engine: PlanEngine` defaults to **`RunnerV2`** (`#[default]` main.rs:1300-1303). The handler only enters the Graph branch when `matches!(engine, PlanEngine::Graph)` (plan.rs:258) — i.e. only with an explicit `--engine graph`. The inline comment "Graph Engine path (default)" (plan.rs:257) is **stale/misleading**: with no flag, `engine == RunnerV2`, the Graph branch is skipped, and control falls into the "Runner v2 path" block (plan.rs:269+) → `roko_cli::runner::event_loop::run(...)` (plan.rs:654; same call do_cmd.rs:616).

**Runner v2 prompt build:** `event_loop::run` dispatches each task via the `Dispatcher` (dispatch/mod.rs:114) whose `prompt_assembler: PromptAssembler` is the **CLI-side** `PromptAssembler` in `dispatch/prompt_builder.rs:717` (constructed `PromptAssembler::new()` factory.rs:87). Its `assemble()` (prompt_builder.rs:774-996) hand-rolls markdown sections — `# Role`, `# Task`, `# Files in scope`, `# Acceptance criteria`, `# Verify`, `# Allowed tools`, `# Prior Task Outputs`, `# PRD Requirements`, workspace/tasks_toml/cfactor blocks — then `enforce_budget()` (prompt_builder.rs:1011+) drops by priority. **It never constructs `SystemPromptBuilder`, `RoleSystemPromptSpec`, or `PromptComposer`.** Its own doc comment concedes this: "9-layer `SystemPromptBuilder` is exposed as a follow-up — see `.roko/GAPS.md`" (prompt_builder.rs:714-715). `event_loop.rs:4387` consumes `dispatch_plan.prompt.system_prompt` straight from that output.

**Consequence:** on the default path, the entire canonical stack — 12 slots, U-shape placement, VCG/greedy auction, affect modulation via `PadState`, pheromones, section-effectiveness priority bumps, cache markers — **does not run.** Those only fire on the two *non-default* paths below (orchestrate inline + `RoleSystemPromptSpec`). Section-effectiveness *is* partially re-implemented CLI-side (`apply_section_effectiveness` prompt_builder.rs:927), but as a parallel copy, not the compose one.

**Where the canonical builder actually runs:**
- `orchestrate.rs::dispatch_agent_with` — builds sections inline + `PromptComposer::new()` (orchestrate.rs:16377). This is the legacy DAG executor path (`roko run`, and older orchestrate entrypoints), NOT the default `plan run`.
- `RoleSystemPromptSpec::build` (role_prompts.rs:451-453) → `builder_with_section_effectiveness` (role_prompts.rs:401-447) → real `SystemPromptBuilder`. Callers: prompting.rs:38-96, run.rs:1517, dispatch_helpers.rs:133, prompt_helpers.rs:94, orchestrate.rs:20545.

## 3. Assembly-surface census — SIX parallel surfaces

| # | Surface | File:line | Delegates to canonical builder? | Live callers | Status |
|---|---|---|---|---|---|
| A | `SystemPromptBuilder` (12-slot builder) | system_prompt_builder.rs:62 | — (is the canonical builder) | via `RoleSystemPromptSpec` + orchestrate inline | ✅ wired (non-default paths) |
| B | `PromptComposer` (budget knapsack + auction) | prompt.rs:781 | consumes builder `PromptSection`s | orchestrate.rs:16377; `RoleSystemPromptSpec::compose_sections_to_build` role_prompts.rs:643-681 | ✅ wired (non-default paths) |
| C | `RoleSystemPromptSpec::build*` (role→builder+composer wrapper) | role_prompts.rs:401-468 | **yes** (A+B) | prompting.rs, run.rs, dispatch_helpers.rs, prompt_helpers.rs | ✅ wired |
| D | **`PromptAssembler` (CLI runner-v2)** | dispatch/prompt_builder.rs:717 | **NO — self-contained shortcut** | **Runner v2 event_loop (the DEFAULT `plan run`)** | 🟡 live but bypasses canonical stack |
| E | `PromptAssembler` (compose templates) | templates/assembly.rs:150-204 | no (own knapsack over `RolePromptTemplate::sections`) | **zero** (grep in roko-cli → 0 hits) | 🔌 built-not-wired |
| F | `PromptAssemblyService` (impls `roko_core::foundation::PromptAssembler`) | prompt_assembly_service.rs | delegates to builder internally | roko-orchestrator `service_factory.rs:210,233`; roko-runtime `workflow_engine.rs:1323` | 🟡 partial (engine/service path) |

Plus two degenerate one-offs: roko-serve `dispatch.rs:1999` builds `SystemPromptBuilder::new(prompt).build()` (1 layer), and the roko-graph **`ComposeCell`** (compose.rs:64) does dumb `{{var}}` string substitution (compose.rs:104-128) — **not** roko-compose at all; the roko-graph cognitive-loop `system-prompt-builder` cell is still a `PassthroughCell` stub (stubs.rs:72). The `--engine graph` path therefore also bypasses the canonical builder (its `AgentCell` reads a static `system_prompt` from node config, agent.rs:82-111).

**Net:** none of the three plan-execution engines (Runner v2 default, Graph, orchestrate legacy) share one prompt surface. Runner v2 (default) uses D; Graph uses ComposeCell/static; orchestrate uses A+B+C.

## 4. VCG-vs-greedy decision path + exact warmup threshold

Threshold constant: `DEFAULT_VCG_WARMUP_OBSERVATIONS = 10` (strategy.rs:10).

Resolution (strategy.rs:28-58):
```
CompositionStrategy::Auto.resolve(bidder_observations, warmup)
  → auto_select: min_obs = bidder_observations.values().min().unwrap_or(0)
  → if min_obs >= warmup (10) { Vcg } else { DensityGreedy }
```
`WeightedSum` always collapses to `DensityGreedy`; explicit `Vcg`/`DensityGreedy` pass through.

**Why it can never reach VCG at runtime:**
1. `PromptComposer::new()` is constructed with an **empty** `learning_bidders` map (prompt.rs:633-637; orchestrate.rs:16377). `bidder_observations` is therefore empty → `min().unwrap_or(0)` = 0 < 10 → `DensityGreedy`, 100% of the time.
2. The observation counter only advances via `update_with_cost` / `update_bidders`. Workspace-wide, the **only** callers are inside prompt.rs itself (foraging pre-pass prompt.rs:715, which is dormant — `with_foraging` never called) and unit tests (prompt.rs:2126-2130, auction.rs:531-542). **No orchestrate / event_loop / gate-verdict caller exists.** So even the non-default canonical path never warms the bidders.
3. `with_learning_bidders` / `with_strategy` have **zero non-test callers** (grep confirmed).

The greedy path still computes a VCG **payment summary** for diagnostics (prompt.rs:1254-1264) and emits a `CompositionManifest`, so signals *look* auction-aware while allocation is pure density-greedy (diversity boost 1.18, diminishing `0.82^wins`, affect multiplier).

**To close the loop:** persist `LearningBidder`s (e.g. `.roko/learn/attention-bidders.json`), construct the composer with `with_learning_bidders`, and call `update_with_cost(section, included, gate_passed, cost, tokens)` after each gate verdict — on **whichever surface becomes canonical** (today that must include surface D, or D must be retired in favour of C).

## 5. Templates enumeration

**Role prompt templates** — 11 structs, all `impl RolePromptTemplate` (trait templates/mod.rs:82), one file each under `crates/roko-compose/src/templates/`:

| Template | File:line (impl) | Role(s) served |
|---|---|---|
| `ConductorTemplate` | conductor.rs:93 | Conductor |
| `ImplementerTemplate` | implementer.rs:83 | Implementer |
| `IntegrationTemplate` | integration.rs:54 | IntegrationTester |
| `QuickReviewerTemplate` | quick.rs:66 | QuickReviewer |
| `QuickFixTemplate` | quick.rs:179 | AutoFixer |
| `RefactorerTemplate` | refactorer.rs:99 | Refactorer |
| `ResearcherTemplate` | researcher.rs:105 | Researcher |
| `ReviewerTemplate` (variants `Reviewer::Architect`/`Auditor`) | reviewer.rs:111 | Architect, Auditor |
| `ScribeTemplate` (variants Initial/Critic) | scribe.rs:108 | Scribe, Critic |
| `StrategistTemplate` | strategist.rs:69 | Strategist |
| `TaskImplTemplate` | task_impl.rs:89 | task-level implementer |

Support modules: `assembly.rs` (dormant surface E), `common.rs` (`PromptBudget`, `adaptive_budget_for`, `REFERENCE_CONTEXT_WINDOW_TOKENS`), `mod.rs` (re-exports + trait). Role→template mapping is `role_identity_for` (role_prompts.rs:717-744); provenance is `role_prompt_source_for` (role_prompts.rs:748+, all `roko_owned:true`).

**Enrichment prompt constants** — `templates/prompts.rs` holds the 13-step pipeline's system+user prompt pairs (distinct from role templates, consumed by the enrichment pipeline not the builder): `PRD_SYSTEM` (prompts.rs:43), `BRIEF_SYSTEM` (:82), `TASKS_SYSTEM` (:131), `DECOMPOSE_SYSTEM` (:178), `RESEARCH_SYSTEM` (:234), `DEPENDENCIES_SYSTEM` (:271), `FIXTURES_SYSTEM` (:299), `INTEGRATION_SYSTEM` (:332), `VERIFY_SYSTEM` (:371), `REVIEW_SYSTEM` (:432), `TESTS_SYSTEM` (:504), `INVARIANTS_SYSTEM` (:540), `SCRIBE_SYSTEM` (:588) — each with a matching `*_user(...)` fn. Budgets `PLAN_BUDGET=30_000` / `SUPPORT_BUDGET=8_000` (prompts.rs:11-13).

## 6. Checklist — unify on one surface + close the VCG loop

- [ ] **[P0]** Decide the canonical assembly surface. Recommended: make Runner-v2's `Dispatcher` build prompts via `RoleSystemPromptSpec::build` (surface C→A+B) instead of the hand-rolled CLI `PromptAssembler` (surface D). Verify: `grep -n 'RoleSystemPromptSpec\|SystemPromptBuilder' crates/roko-cli/src/dispatch/prompt_builder.rs` (expect >0).
- [ ] **[P0]** Delete or forward surface D's markdown authorship to the 12-slot builder so the default `plan run` gains U-shape/affect/pheromones/auction. Verify: run `roko plan run` and confirm `.roko/signals.jsonl` carries a `composition_manifest` tag (currently absent on runner-v2).
- [ ] **[P0]** Close VCG warmup: persist `LearningBidder`s, construct composer `with_learning_bidders`, call `update_with_cost(...)` after each gate verdict. Verify: `roko plan run plans/ && grep -c '"selected_strategy":"vcg"' .roko/signals.jsonl` (>0 after ~10 tasks).
- [ ] **[P1]** Fix the stale comment (plan.rs:257 "Graph Engine path (default)" — default is RunnerV2) and reconcile `PlanEngine` default vs docs. Verify: `grep -n '#\[default\]' crates/roko-cli/src/main.rs`.
- [ ] **[P1]** Retire dormant surface E (`templates/assembly.rs::PromptAssembler`, zero callers) or route it behind C. Verify: `grep -rn 'assembly::PromptAssembler\|TemplateAssembly' crates/roko-cli/src` (expect 0 → then remove).
- [ ] **[P1]** Add `[prompt] composition_strategy` + `vcg_warmup_observations` config knobs feeding `with_strategy`. Verify: `roko config show | grep composition_strategy`.
- [ ] **[P2]** Replace roko-graph `ComposeCell` string-substitution + `system-prompt-builder` `PassthroughCell` with a real cell delegating to `RoleSystemPromptSpec::build_sections`, so `--engine graph` also uses the canonical stack. Verify: `grep -n 'PassthroughCell' crates/roko-graph/src/cells/stubs.rs` (expect the compose entry gone).
- [ ] **[P2]** De-duplicate section-effectiveness (compose `effective_priority` vs CLI `apply_section_effectiveness` prompt_builder.rs:927) once surfaces merge. Verify: single implementation reachable from the canonical path.
- [ ] **[P3]** Fix `layer_count()` to include `tool_hints` and return 12; align header "9 layers" wording. Verify: `cargo test -p roko-compose layer_count`.

## 7. Roadmap
1. **Unify (P0):** one surface — collapse D and orchestrate-inline onto C/A+B; graph cell (P2) delegates to the same. Removes 3 divergent prompt shapes.
2. **Instrument (P0):** wire the bidder observation loop from gate verdicts so `Auto` can cross the 10-observation threshold; ship VCG behind an `ExperimentStore` A/B before default-on.
3. **Configure (P1):** expose strategy/warmup/foraging/dedup via `[prompt]` config.
4. **Consolidate types (P2):** one `GateFeedback` (compose/gate/runner triplicate).
5. **Protocolize (P3):** define v2 `ComposeProtocol`/`ComposeBid`/`ComposeResult`; the bid-shaped internals (`AuctionCandidate`, `VcgBid`, `CompositionManifest`) already model the data.
