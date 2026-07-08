# Prompt: 16-heartbeat

You are a fresh Claude Opus agent. Zero prior context. Read every file this prompt references before writing.

## Your mission

Generate `/Users/will/dev/nunchi/roko/roko/docs/16-heartbeat/`. Covers the CoALA 9-step cognitive pipeline and how it maps to the universal Synapse loop, three cognitive speeds (Gamma/Theta/Delta), adaptive clock, gating, context governor, attention auctions (VCG), sleepwalker 3-step variant, CorticalState, dual-process (T0/T1/T2), active inference for compute allocation, 16 T0 probes.

## Step 1 — Context pack (MANDATORY)

Read all 7 files in `/Users/will/dev/nunchi/roko/roko/tmp/prd-migration/context-pack/` in order.

## Step 2 — refactoring-prd canonical sources

1. `/Users/will/dev/nunchi/roko/refactoring-prd/01-synapse-architecture.md` §3 Universal Cognitive Loop, §Three Cognitive Speeds, §Dual-Process Cognition, §4 Active Inference Integration
2. `/Users/will/dev/nunchi/roko/refactoring-prd/03-cognitive-subsystems.md` all sections (subsystems drive the heartbeat)
3. `/Users/will/dev/nunchi/roko/refactoring-prd/02-five-layers.md` §Adaptive Clock (L0 Runtime)
4. `/Users/will/dev/nunchi/roko/refactoring-prd/09-innovations.md` §I 16 Zero-Cost Cognitive Probes (T0 Layer), §II VCG Attention Auction, §XIX.A Active Inference State Space (factorized POMDP)
5. `/Users/will/dev/nunchi/roko/refactoring-prd/05-agent-types.md` §3 CoALA Heartbeat mapping (chain variant)
6. `/Users/will/dev/nunchi/roko/refactoring-prd/08-translation-guide.md` §7 Golem Heartbeat → Universal Loop

## Step 3 — SOURCE-INDEX entry `## 16-heartbeat.md`

Key legacy:
- `bardo-backup/prd/01-golem/01-cognition.md`, `02-heartbeat.md`, `03-mind.md`, `03b-cognitive-mechanisms.md`, `03c-state-management.md`, `14-context-governor.md`, `14b-attention-auction.md`, `15-sleepwalker.md`, `16-risk-engine.md`, `17-prediction-engine.md`, `18-cortical-state.md`
- `bardo-backup/prd/12-inference/01-deployment-modes.md`, `01a-routing.md`, `15-inference-profiles.md`
- `bardo-backup/tmp/mori-refactor/12-cognitive-architecture.md`, `03-runtime.md` (adaptive clock)
- `bardo-backup/tmp/agent-chain/01-overview.md` (CoALA mapping)

## Step 4 — implementation-plans

- `12a-cognitive-layer.md` §I Operating Frequencies (I1–I5 Gamma/Theta/Delta loops, frequency scheduler, meta-cognition hook)

## Step 5 — active code

- Read `/Users/will/dev/nunchi/roko/roko/crates/bardo-runtime/src/` (to rename roko-runtime) — adaptive clock, event bus
- Read `/Users/will/dev/nunchi/roko/roko/crates/bardo-primitives/src/tier.rs` — InferenceTier, TierRouter
- Read `/Users/will/dev/nunchi/roko/roko/crates/roko-learn/src/cascade_router.rs`

## Step 6 — Output and sub-doc plan

```bash
mkdir -p /Users/will/dev/nunchi/roko/roko/docs/16-heartbeat
```

Write **13 sub-docs** plus `INDEX.md`:

| # | Filename | Content |
|---|---|---|
| 00 | `00-coala-9-step-pipeline.md` | CoALA framework (Sumers et al. 2023, arXiv:2309.02427). The 9 steps: OBSERVE → RETRIEVE → ANALYZE → GATE → SIMULATE → VALIDATE → EXECUTE → VERIFY → REFLECT. Per-step responsibilities. ACT-R and SOAR as predecessors. Why CoALA is the organizing framework. |
| 01 | `01-universal-loop-mapping.md` | How CoALA 9-step maps to the universal Synapse loop (PERCEIVE/EVALUATE/ATTEND/INTEGRATE/ACT/VERIFY/PERSIST/ADAPT/META-COGNIZE). Side-by-side table. The universal loop is the domain-agnostic version; chain heartbeat is a domain-specific parameterization. |
| 02 | `02-chain-heartbeat-variant.md` | Chain agents add SIMULATE (mirage-rs pre-flight) and VALIDATE (position limits) steps between ATTEND and ACT. Why — chain actions are financially irreversible, require pre-flight verification. Full mapping table. Cross-reference 08-chain.md §19. |
| 03 | `03-three-cognitive-speeds.md` | Gamma (~5-15s reactive), Theta (~75s reflective), Delta (hours consolidation). Named after EEG frequency bands. Each timescale runs concurrently on separate async tasks. All three run simultaneously, not sequentially. |
| 04 | `04-gamma-reactive-loop.md` | Main orchestration loop. Tool calls, LLM inference, verification. Triggered continuously by async event loop. Current orchestration mode. |
| 05 | `05-theta-reflective-loop.md` | Fires periodically (every 5 gamma cycles or every episode completion). Summarize recent work. Update Daimon state. Check predictions. "Step back and think about the plan." |
| 06 | `06-delta-consolidation-loop.md` | Hours-scale. Dreams replay. Knowledge distillation. Tier promotion. Triggered by idle detection or scheduled (nightly). Cross-reference 10-dreams.md. |
| 07 | `07-adaptive-clock.md` | `roko-runtime` adaptive clock. Three timescales managed by runtime. Gamma 5-15s (faster when issues detected). Theta 30-120s (regime multipliers adjust interval). Delta ~50 theta cycles (strategic, fires periodically during idle). |
| 08 | `08-dual-process-t0-t1-t2.md` | System 1 / System 2 (Kahneman, CLARION). T0 (direct tool call, no LLM — ~80% of ticks). T1 (fast model, shallow — ~15%). T2 (full model, deep — ~5%). Routing from prediction error. Uncertainty-driven, not manual thresholds. |
| 09 | `09-16-t0-probes.md` | The 16 zero-LLM probes from 09-innovations.md §I. Each is a pure function `fn probe(state: &EngineState) -> f32`. Blockchain probes: price delta, TVL delta, position health, gas spike, credit balance, RSI, MACD, circuit breaker. Coding probes: build health, test regression, complexity drift, dependency risk, coverage delta, error rate. Universal: world model drift, causal consistency. Weighted sum → prediction error scalar → tier routing. FrugalGPT (Chen et al. 2023, arXiv:2305.05176) for cost savings. Extensibility via `Vec<Box<dyn Probe>>`. |
| 10 | `10-active-inference-compute-allocation.md` | Expected Free Energy (EFE) decomposes into pragmatic value (goal-directed) + epistemic value (information-seeking) - ambiguity. High uncertainty → epistemic dominates → explore. Low uncertainty → pragmatic dominates → exploit. **Zero hyperparameters for explore/exploit.** Agent's own uncertainty determines compute investment. Full EFE formula. Parr et al. 2024, Koudahl et al. 2024. |
| 11 | `11-active-inference-state-space.md` | Factorized discrete POMDP from 09-innovations.md §XIX.A (Koudahl et al. 2024, VERSES Genius). Don't model the world, model the agent's **epistemic situation**: 6 task phases × 5 context quality × 3 uncertainty = 90 tractable states. A/B/C/D matrices from pymdp. LLM operates inside the active inference framework — active inference decides what goes in context, LLM processes. |
| 12 | `12-attention-auction-and-gating.md` | VCG attention auction (cross-reference 03-composition.md §10). Context governor for token allocation governance. Gating: when to suppress/escalate ticks. Sleepwalker 3-step variant for sleep mode (reduced CoALA steps). CorticalState and cognitive state management. Meta-cognition hook ("Am I stuck? Should I escalate?"). Frequency scheduler decides which loop runs. |

Plus `INDEX.md`.

## Step 7-9 — Rules, INDEX, self-check

Per context-pack rules. ≥200 lines per sub-doc, ≥3000 total. Citations: Sumers et al. 2023 CoALA (arXiv:2309.02427), Kahneman Thinking Fast and Slow, CLARION Sun et al., FrugalGPT Chen et al. 2023 (arXiv:2305.05176), Friston FEP, Parr et al. 2024 (arXiv:2402.14460), Koudahl et al. 2024 (arXiv:2412.10425), pymdp.

Cross-reference 00-architecture, 02-agents (tier routing), 03-composition (VCG), 09-daimon (behavioral state → tier bias), 10-dreams (Delta loop).

## CRITICAL REMINDERS

- DO NOT SUMMARIZE. DO NOT TRUNCATE.
- The 9-step cognitive loop and the 3 cognitive speeds are the core of this topic. Be thorough.
- T0 probes are zero-LLM pure functions — cite FrugalGPT for the cost-reduction thesis.
- Apply naming map: golem → agent.
- Use Write tool. Don't ask questions.
