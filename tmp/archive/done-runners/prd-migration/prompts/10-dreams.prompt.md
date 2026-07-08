# Prompt: 10-dreams

You are a fresh Claude Opus agent. Zero prior context. Read every file this prompt references before writing.

## Your mission

Generate `/Users/will/dev/nunchi/roko/roko/docs/10-dreams/`. Covers the 3-phase dream cycle (NREM replay + REM imagination + integration staging), Mattar-Daw prioritized replay, Boden creativity modes, Pearl SCM counterfactuals, emotional depotentiation, HDC counterfactual synthesis, SQLite staging buffer, sleep-time compute, hypnagogia engine (solves Alpha Convergence Problem), Oneirography, Venice dreaming. **NO death proximity triggers.**

## Step 1 — Context pack (MANDATORY)

Read all 7 files in `/Users/will/dev/nunchi/roko/roko/tmp/prd-migration/context-pack/` in order.

## Step 2 — refactoring-prd canonical sources

1. `/Users/will/dev/nunchi/roko/refactoring-prd/03-cognitive-subsystems.md` §3 Dreams
2. `/Users/will/dev/nunchi/roko/refactoring-prd/09-innovations.md` §IV Alpha Convergence & Hypnagogia (concrete LLM recipe), §V Dream Engine 3-phase (NREM Mattar-Daw + REM Boden+Pearl + Integration), §XIX.G Dream Scheduling (idle-time, cheap model, concurrent)
3. `/Users/will/dev/nunchi/roko/refactoring-prd/08-translation-guide.md` §**INCOMPATIBLE: Dream as Approaching Death**
4. `/Users/will/dev/nunchi/roko/refactoring-prd/07-implementation-priorities.md` §Tier 2F/2G (Dreams consolidation, counterfactual simulation via HDC permutation)

## Step 3 — SOURCE-INDEX entry `## 10-dreams.md`

Read every file. Key legacy:
- `bardo-backup/prd/05-dreams/00-overview.md`, `01-architecture.md`, `01b-dream-evolution.md`, `02-replay.md`, `03-imagination.md`, `04-consolidation.md`, `05-threats.md`, `06-integration.md`, `07-venice-dreaming.md`
- `bardo-backup/prd/06-hypnagogia/00-overview.md`, `01-neuroscience.md` (full neuroscience basis), `02-architecture.md`, `03-divergence-alpha.md`, `04-homunculus.md`, `05-hauntology.md` (Derrida traces), `06-xenocognition.md`, `07-inner-worlds.md`
- `bardo-backup/prd/22-oneirography/00-overview.md`, `01-dream-journals.md`, `03-self-appraisal.md`, `04-auctions.md`, `05-extended-forms.md`

**SKIP ENTIRELY**: `bardo-backup/prd/22-oneirography/02-death-masks.md`.

## Step 4 — implementation-plans

- `12a-cognitive-layer.md` §G Dreams (G1–G8: replay scheduler trigger on idle, mistake identification, heuristic strengthen/weaken, dreams→neuro pipeline, counterfactual simulation via HDC permutation, cross-episode consolidation via HDC bundling, novel strategy generation)
- `12a-cognitive-layer.md` §R3 roko-dreams crate creation

## Step 5 — active code

- Read `/Users/will/dev/nunchi/roko/roko/crates/roko-dreams/src/lib.rs` (scaffold)
- Read `/Users/will/dev/nunchi/roko/roko/crates/roko-golem/src/dreams.rs` (43-line placeholder to delete)
- Read `/Users/will/dev/nunchi/roko/roko/crates/roko-golem/src/hypnagogia.rs` (42-line placeholder to move to roko-dreams)
- Read `/Users/will/dev/nunchi/roko/roko/crates/roko-learn/src/hdc_clustering.rs` (K-medoids)
- Read `/Users/will/dev/nunchi/roko/roko/crates/roko-learn/src/pattern_discovery.rs` (trigram mining)

## Step 6 — Output and sub-doc plan

```bash
mkdir -p /Users/will/dev/nunchi/roko/roko/docs/10-dreams
```

Write **17 sub-docs** plus `INDEX.md`:

| # | Filename | Content |
|---|---|---|
| 00 | `00-vision-and-dream-as-death-reframe.md` | Dreams as Delta-frequency cognitive process. **Explicit reframe from legacy "dream as approaching death" to "idle-time / scheduled consolidation."** Intensity based on volume of unprocessed episodes, NOT death proximity. Why dreams matter (catastrophic forgetting prevention, novel hypothesis generation, Alpha Convergence Problem solution). |
| 01 | `01-six-step-dream-cycle.md` | REPLAY → CONSOLIDATE → PRUNE → SYNTHESIZE → VALIDATE → OPTIMIZE. Each step in detail. Time budget per step. |
| 02 | `02-nrem-replay-mattar-daw.md` | Phase 1 (8-15 min). Mattar-Daw utility formula: `Utility(episode) = Gain × Need × (1/spacing_penalty)`. Gain = prediction error magnitude. Need = frequency of similar situations. Spacing penalty = Ebbinghaus spacing effect. 30% perturbed replays (2× slippage, 5× gas, correlation shifts for stress testing). PAD modulates selection (anxious → 2× warning episode weight). Citation: Mattar & Daw 2018 "Prioritized memory access explains planning and hippocampal replay." |
| 03 | `03-rem-imagination-boden-pearl.md` | Phase 2 (5-15 min). Counterfactual scenario generation. **Boden's three creativity modes**: Combinational (recombine knowledge) / Exploratory (traverse known boundaries) / Transformational (break constraints). Implemented via **Pearl's structural causal models** — build causal graph, intervene on variables. Emotional depotentiation (Walker & van der Helm 2009) reduces arousal on charged memories by 0.3-0.5/cycle to prevent panic lock-in. |
| 04 | `04-hdc-counterfactual-synthesis.md` | HDC permutation operations generate novel knowledge combinations in nanoseconds — far faster than LLM-based generation. Bundle multiple insights, permute positions, check Hamming similarity to known successful patterns. |
| 05 | `05-integration-staging-buffer.md` | Phase 3 (5-10 min). SQLite staging buffer. Hypotheses enter at 0.20-0.30 confidence. Only those reaching 0.70 through live validation get promoted to permanent memory. Prevents hallucinated insights from corrupting knowledge base. |
| 06 | `06-what-dreams-produce.md` | Knowledge promotions (working-tier → consolidated). Pattern discovery → Warnings. Skill extraction → StrategyFragments. Hypothesis generation via HDC bundling. Routing updates. Prompt optimization. |
| 07 | `07-neuroscience-basis.md` | Complementary Learning Systems (McClelland 1995). Hippocampal replay (Wilson & McNaughton 1994). Non-veridical replay aids generalization. Beneficial forgetting. World Models (Ha & Schmidhuber 2018). DreamerV3 (Hafner 2025). |
| 08 | `08-sleep-time-compute.md` | Lin et al. 2025 — 5× reduction in test-time compute via overnight consolidation. WSCL 2024 — 38% reduction in catastrophic forgetting. Biological sleep research connections. |
| 09 | `09-hypnagogia-alpha-convergence.md` | **The Alpha Convergence Problem**: all AI agents using the same foundation models → same analyses → alpha → 0 in competitive domains. Hypnagogia is Roko's solution: force experiential divergence. Each agent "differently haunted" (Derrida 1993, hauntology) by its own experiential traces. |
| 10 | `10-hypnagogia-four-layers.md` | ThalalamicGate (progressively reduce external input, redirect attention inward). ExecutiveLoosener (temperature annealing T=1.3-1.5 ideation → T=0.3-0.5 evaluation; min-p sampling). DaliInterrupt (capture 3-5 partial completions at peak temperature before LLM reaches conclusion — the Edison/Dali bottle-drop technique). HomuncularObserver (evaluate fragments at T=0.4 on novelty/relevance/coherence — filters noise from insight). |
| 11 | `11-hypnagogia-llm-recipe.md` | From 09-innovations.md §IV, the concrete LLM implementation recipe. Step 1: Thalamic Gate — HDC-encode recent 5 episodes, bundle → fingerprint, retrieve 3-5 entries with LOWEST similarity (anti-correlated). Step 2: Executive Loosener — LLM call 1 at T=1.3, top_p=0.95, min_p=0.02 → 5 hypotheses. Step 3: Dali Interrupt — partial completions at T=1.0, stop at 50-100 tokens, 3-5× per hypothesis → 15-25 fragments. Step 4: Homuncular Observer — LLM call 2 at T=0.4 structured output, rate (novelty > 0.5 AND relevance > 0.3 AND coherence > 0.4), typically 3-7 survive. Cost: ~2,000-4,000 tokens ~$0.01. |
| 12 | `12-hypnagogia-citations.md` | Lacaux et al. 2021 Science Advances: 83% hidden rule discovery in N1 stage vs 30% staying awake. Haar Horowitz et al. 2020 (MIT Dormio project) and 2023 (43% creativity boost via targeted dream incubation). Derrida 1993 (Specters of Marx, hauntology). Edison's bottle-drop technique. Dali's key drop. Full neuroscience of hypnagogia. |
| 13 | `13-sleepwalker-mode.md` | Reduced-capability sleep mode. 3-step variant of CoALA (vs full 9-step). Keeps agent minimally responsive while offline learning runs. |
| 14 | `14-oneirography.md` | Dream journals, self-appraisal, extended forms. What gets logged. Auctions on dream outputs. Per `bardo-backup/prd/22-oneirography/`. **SKIP death-masks** — do not carry that concept forward. |
| 15 | `15-dream-scheduling.md` | Dreams don't block the agent. Run concurrently. NREM replay uses Haiku-class cheap model. REM imagination uses Sonnet-class. Integration is pure computation (no LLM). Parallel execution: resting state → spawn dream engine on separate async task. On task arrival, SIGPAUSE dream, serialize, resume later. Impact on throughput: ~0% when tasks available. Can also be scheduled nightly. |
| 16 | `16-current-status-and-gaps.md` | roko-dreams scaffold. roko-golem/dreams.rs 43-line placeholder (to delete after R3). roko-golem/hypnagogia.rs 42-line placeholder (to move to roko-dreams). Wiring gaps from 12a §G (G1-G8). roko-learn has trigram mining and K-medoids already built — integration needed. |

Plus `INDEX.md`.

## Step 7-9 — Rules, INDEX, self-check

Per context-pack rules. ≥200 lines per sub-doc, ≥4000 total. Preserve all dream/sleep citations: McClelland 1995, Wilson & McNaughton 1994, Mattar & Daw 2018, Ha & Schmidhuber 2018, Hafner 2025 DreamerV3, Lin et al. 2025 (sleep-time compute), WSCL 2024, Lacaux et al. 2021 Science Advances, Haar Horowitz 2020/2023 Dormio, Derrida 1993, Walker & van der Helm 2009, Park 2023 (arXiv:2304.03442 Generative Agents reflection), Boden creativity theory, Pearl 2009 Causality (SCM).

Cross-reference topics 00-architecture (Delta frequency), 06-neuro (dream outputs feed neuro), 09-daimon (PAD modulates replay, emotional depotentiation), 16-heartbeat (dreams are the Delta loop).

## CRITICAL REMINDERS

- DO NOT SUMMARIZE. DO NOT TRUNCATE. PRESERVE ALL DREAM/SLEEP CITATIONS.
- **NO death proximity triggers**. Dreams are triggered by idle time or schedule, NOT by approaching death. Legacy sources frame dreams as "preparation for death" — REWRITE to "idle-time consolidation."
- Hypnagogia is **Roko's solution to the Alpha Convergence Problem** — make this the main framing for the hypnagogia sub-docs.
- The Edison/Dali bottle-drop technique is real and is the basis for DaliInterrupt. Cite it.
- SKIP `22-oneirography/02-death-masks.md` entirely.
- Apply naming map: golem → agent.
- Use Write tool. Don't ask questions.
