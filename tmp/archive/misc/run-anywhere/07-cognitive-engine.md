# The Cognitive Engine: Heartbeat, Affect, Dreams, and Hypnagogia

> **Audience**: Researchers, architects, developers understanding roko's cognitive mechanisms
> **Scope**: The four inner systems that make roko agents think, feel, imagine, and create
> **Note**: Mechanisms formerly named "mortality/daimon/dreams/hypnagogia" in the Bardo PRDs.
> The mechanisms are preserved; naming updated for roko context.

---

## 1. The Heartbeat: Decision Cycles, Not Chat Turns

### Three Concurrent Timescales

Roko agents don't tick on a fixed clock. They operate at three adaptive frequencies, inspired by neural oscillation bands (Buzsaki, 2006):

| Frequency | Period | Purpose | Cost |
|---|---|---|---|
| **Gamma** (~5-15s) | Perception | Resolve pending predictions, update shared state, check attention promotions | Near-zero (no LLM) |
| **Theta** (~30-120s) | Full cognition | Predict → appraise → gate → [retrieve → deliberate → act] → reflect | Variable (T0/T1/T2) |
| **Delta** (~50 theta ticks) | Consolidation | Curator cycle, statistics aggregation, attention rebalancing, dream scheduling | Low (batch processing) |

**Adaptive scaling**: Gamma accelerates when violations are detected. Theta accelerates during market volatility. Both throttle toward daily budget ceilings.

### The 9-Step Heartbeat Pipeline

Each theta tick executes:

```
1. OBSERVE    → Read environment (on-chain state, file changes, signals)
2. RETRIEVE   → Query neuro (episodic + semantic + holographic)
3. ANALYZE    → LLM reasoning if T1/T2 (gated by prediction accuracy)
4. GATE       → Prediction-accuracy-based action permission
5. SIMULATE   → Pre-flight simulation (Revm fork for DeFi, dry-run for code)
6. VALIDATE   → Safety checks (capability tokens, PolicyCage, path policy)
7. EXECUTE    → On-chain action or file modification
8. VERIFY     → Outcome confirmation (tx receipt, test result, gate verdict)
9. REFLECT    → Learning update (episode log, skill extraction, calibration)
```

### 16 Deterministic Probes (T0, Zero LLM Cost)

At every gamma tick, 16 fast probes run with no LLM involvement:

| Probe | What It Checks | Cost |
|---|---|---|
| Price delta | Significant price movement | $0 |
| TVL delta | Liquidity changes in monitored pools | $0 |
| Position health | Health factor of lending positions | $0 |
| Gas spike | Gas price exceeds threshold | $0 |
| Credit balance | Budget remaining vs burn rate | $0 |
| RSI | Relative Strength Index | $0 |
| MACD | Moving Average Convergence Divergence | $0 |
| Circuit breaker | Protocol-level circuit breaker triggered | $0 |
| Kill switch | Owner-initiated emergency halt | $0 |
| Replicant report | Sibling agent alerts | $0 |
| Clade alert | Clade-wide signals | $0 |
| Homeostatic drift | Portfolio drifting from target allocation | $0 |
| World model drift | Environment diverging from agent's model | $0 |
| Causal consistency | Causal graph predictions matching reality | $0 |
| VPIN | Volume-synchronized probability of informed trading | $0 |
| IL threshold | Impermanent loss exceeding tolerance | $0 |

These probes compute **prediction error** — a single scalar (0.0-1.0) that drives T0/T1/T2 gating. When all probes are within bounds, prediction error is low → T0 suppresses → no LLM call → $0.

### Five Behavioral Phases (Resource-Driven)

| Phase | Resource Level | Behavior | Inference |
|---|---|---|---|
| **Thriving** | >70% | Aggressive exploration, hypothesis testing, dream scheduling | Full T2 access |
| **Stable** | 50-70% | Balanced exploitation, standard operations | T2 for important decisions |
| **Conservation** | 30-50% | Reduced risk, consolidation, knowledge export | T1 preferred, T2 rare |
| **Declining** | 10-30% | Maximum knowledge extraction | T0+T1 only |
| **Terminal** | <10% | Pure reflection, legacy transfer | T0 only, no spending |

These phases are not heuristics — they are **provably optimal finite-horizon MDP policies** (Altman, 1999). An agent with 10% budget remaining should have a fundamentally different strategy than one with 90%.

**~80% of theta ticks suppress at the GATE step** — the prediction gate determines that no action is needed. This is the T0 tier: no LLM invoked, zero cost.

**Research**: CoALA (Sumers et al., 2023) — cognitive architecture for language agents. Active Inference (Friston, 2010) — agents maintain generative models and act to reduce prediction error.

### The CorticalState (Lock-Free Shared Perception)

A 256-byte atomic structure (~4 cache lines) shared across all subsystems without locks:

| Group | Signals | Written By |
|---|---|---|
| **AFFECT** | pleasure, arousal, dominance, primary_emotion | Daimon/affect engine |
| **PREDICTION** | aggregate_accuracy, accuracy_trend, surprise_rate, pending | Prediction engine |
| **ATTENTION** | universe_size, active_count, watched_count | Attention forager |
| **ENVIRONMENT** | regime, regime_confidence | Domain probes |
| **RESOURCE** | economic_vitality, epistemic_vitality, composite_vitality, phase | Resource manager |
| **INFERENCE** | budget_remaining, current_tier | Inference router |
| **CREATIVE** | creative_depth, dream_phase | Dream engine |
| **DERIVED** | compounding_momentum | Runtime per-tick |

**32 continuously interpolating variables**. No locks — uses `Ordering::Release/Acquire` atomics. Acceptable eventual consistency (safety operates on its own strongly-consistent state).

**What's novel**: No other agent framework has a shared perception surface. Each subsystem (router, safety, learning, affect) sees all signals without coupling to the producing subsystem. The CorticalState is the agent's "proprioception" — awareness of its own internal state.

---

## 2. The Affect Engine: Emotions as Control Signals

### PAD Emotional Model

Every roko agent maintains a three-dimensional emotional state:

- **Pleasure** [-1, +1]: Satisfaction with outcomes. High after success, low after failure.
- **Arousal** [-1, +1]: Urgency/alertness. High under time pressure, low at rest.
- **Dominance** [-1, +1]: Confidence/control. High with strong knowledge, low with uncertainty.

**Research**: Mehrabian & Russell (1974) — PAD dimensional model of emotional space. Barrett (2017) — emotions as constructed predictions about bodily states.

### How Emotions Modulate Behavior

The PAD vector is NOT cosmetic. It's a **control signal** that modulates five subsystems:

| Subsystem | How Affect Modulates It |
|---|---|
| **Memory retrieval** | Mood-congruent bias: anxious state → retrieve cautionary knowledge; confident state → retrieve optimization knowledge (Bower, 1981) |
| **Inference tier** | High arousal → P(T0→T1→T2) increases — "surprise costs more inference budget" |
| **Risk tolerance** | Low dominance → reduce position sizes, increase safety margins |
| **Dream scheduling** | Sustained low pleasure → schedule dream consolidation sooner |
| **Skill selection** | Emotional state biases which skills load (anxious → defensive skills) |

### Appraisal Theory (OCC Model & Two-Mode Generation)

Events are appraised against the agent's goals and expectations using the OCC (Ortony, Clore, Collins) framework:
- Goal-relevant event + positive outcome → Joy (pleasure↑)
- Goal-relevant event + negative outcome → Distress (pleasure↓)
- Unexpected event → Surprise (arousal↑)
- Uncertain threat → Fear (arousal↑, dominance↓)

**Two-Mode Appraisal Generation**:
The affect engine evaluates predictions in two distinct modes depending on the current inference tier:
- **Mode A (Deterministic T0)**: Rule-based. Translates P&L direction to pleasure, prediction error magnitude to arousal, and survival phase to dominance. Zero LLM cost. 
- **Mode B (Chain-of-Emotion T1/T2)**: Piggybacked LLM appraisal. When the agent is already calling Haiku or Sonnet, a structured `<daimon><pad>...</pad></daimon>` output block is requested. Costs ~$0.000006 per appraisal but captures deep contextual attribution (e.g., recognizing a trade succeeded due to luck, producing 'surprise' instead of 'joy').

A Generous Grounding validation (1.0 Euclidean distance in PAD space) ensures the LLM's emotional appraisal doesn't catastrophically hallucinate beyond deterministic realities (e.g., LLM cannot claim "extreme joy" during a massive liquidation).

### The Somatic Landscape (k-d Tree Valence Map)

Standard somatic markers are discrete points ("this exact state felt bad"). But DeFi operates in continuous parameter spaces (LP width, rebalance thresholds, sizes). Roko implements a **Somatic Landscape**: a continuous internal topology mapping emotional valence across an 8-dimensional strategy space.
- Implemented as a `kiddo::KdTree<f64, ValenceAccumulator, 8>`.
- Evaluates emotional safety *before* analytical processing by running k-NN queries on inverse-distance weighted neighbors. 
- Allows the agent to query "What does this region of strategy space *feel* like?" and avoid regions that historically produced sustained negative affect, even before formal logic rejects it.

### Somatic Markers (Pre-Conscious Decision Bias)

**Research**: Damasio (1994) — somatic marker hypothesis. In the Iowa Gambling Task, subjects' skin conductance responded to risky decks **before** conscious awareness of which decks were dangerous.

Roko implements this: emotional signals fire before the deliberation step (step 3 in the heartbeat). The affect state biases which memories are retrieved (step 2), which in turn biases the analysis. Emotions are the **fastest decision signal**, processed before reasoning begins.

**Novel**: Zhang, Naradowsky & Miyao (2024) showed self-emotion changes ~50% of agent decisions. Gadanho (2003) demonstrated 40% fewer collisions with emotion integration vs. cognition alone. Emotional RAG (2024, arXiv:2410.23041) showed emotion-weighted retrieval beats semantic-only across ChatGLM, Qwen, GPT-3.5.

### What's Novel

No other coding agent or DeFi agent uses emotional state as a structural control signal. Standard approaches use:
- Recency + relevance + importance for retrieval (3 factors)
- Roko adds emotional congruence as a 4th factor (Bower, 1981)
- The PAD vector compresses salience into 3 numbers — the LLM sees 50K tokens but the affect state tells it which 500 matter right now

---

## 3. The Dream Engine: Offline Intelligence

### Why Agents Should Dream

Mortal agents cannot afford to learn everything through direct experience. Each real task costs tokens, time, and budget. **Dreaming multiplies learning from scarce experience**: one real task → dozens of replay analyses + counterfactual branches + creative recombinations.

**Research**:
- Sleep consolidation (Wilson & McNaughton, 1994) — hippocampal replay compresses minutes into ~100ms
- Sleep insight (Wagner et al., 2004) — 59% discovered hidden rules after sleep vs. 23% awake
- DreamerV3 (Hafner et al., 2025) — world models trained entirely in imagination outperform methods trained on real data
- Complementary Learning Systems (McClelland et al., 1995) — fast hippocampal capture + slow neocortical extraction

### ALMA Three-Layer Affect Model (Gebhard, 2005)

The PAD vector is computed from three temporal layers with different speeds:

| Layer | Decay Window | Blend Weight | What It Captures |
|---|---|---|---|
| **Personality** | Lifetime | 0.25 | Baseline temperament (inherited from predecessor at 0.5x) |
| **Mood** | ~4 hours (ALMA α) | 0.50 | Sustained emotional trajectory |
| **Emotion** | ~30 seconds | 0.25 | Per-tick reactive response |

**Effective PAD** = personality × 0.25 + mood × 0.50 + emotion × 0.25

Mood dominates because sustained trajectory matters more than per-tick jitter or fixed personality.

**Emotion update rules** (per tick):
- Correct prediction: pleasure +0.05
- Incorrect prediction: pleasure -0.08 (**1.6x negativity bias** — mirrors prospect theory)
- Arousal: `|residual| / expected_residual_magnitude × 0.1`
- Dominance: +0.03 if improving, -0.05 if declining, 0 if flat

**Eight Plutchik primary emotions** map to PAD prototypes via squared Euclidean distance:

| Emotion | Pleasure | Arousal | Dominance |
|---|---|---|---|
| Joy | 0.7 | 0.3 | 0.4 |
| Trust | 0.4 | -0.2 | 0.3 |
| Fear | -0.6 | 0.8 | -0.5 |
| Surprise | 0.0 | 0.9 | -0.3 |
| Sadness | -0.6 | -0.4 | -0.5 |
| Disgust | -0.5 | 0.3 | 0.2 |
| Anger | -0.5 | 0.8 | 0.5 |
| Anticipation | 0.3 | 0.4 | 0.6 |

---

### Three Dream Phases

**Phase 1: NREM Replay (8-15 minutes)**

Replays prioritized past episodes using the Mattar-Daw utility formula:

```
Utility = Gain × Need × (1 - 0.5 × spacing_penalty)

Gain = 0.4 × surprise_factor + 0.3 × outcome_significance + 0.3 × suboptimality_bonus
  where surprise_factor = prediction error magnitude
        outcome_significance = |pnl| / position_size
        suboptimality_bonus = counterfactual regret (if available)

Need = 0.4 × state_similarity + 0.3 × regime_similarity + 0.3 × recency_decay
  where state_similarity = cosine(episode.market_state, current_state)
        regime_similarity = 1.0 if same regime, 0.3 if different
        recency_decay = exp(-Δt / RECENCY_HALFLIFE)

spacing_penalty = exp(-Δt_last_replay / SPACING_HALFLIFE)
  (prevents fixation on narrow episode set)
```

**Gain decays with replays**: `gain_effective = gain × 0.85^replay_count`. After 5-10 replays, nothing new to learn.

**Episode selection (two-pass)**:
1. Sort all episodes by utility, select top N
2. Reserve 20% for diversity: at least 1 per regime, at least 1 from oldest third, at least 1 high-arousal

**Three replay modes**:
- **Forward**: Planning rehearsal — project from current state, identify likely developments, optimal responses, worst-case bailout plans. Runs at end of NREM phase.
- **Reverse**: Credit assignment — trace outcome ← final action ← hold decision ← entry decision ← analysis. Which step contributed most to the outcome? (Foster & Wilson, 2006; Ambrose et al., 2016)
- **Perturbed** (30% of replays): Stress testing with injected noise:

| Perturbation | What's Injected | Purpose |
|---|---|---|
| Slippage | 2x slippage | Execution robustness |
| Latency | +3 blocks delay | Network robustness |
| Gas spike | 5x gas cost | Fee volatility |
| Data dropout | 10min price gap | Data gap robustness |
| Liquidity drain | 50% pool drop | Thin market robustness |
| Correlation shift | ETH-BTC decoupling | Correlation breakdown |

**PAD modulates replay selection**:

| Emotional State | Replay Bias |
|---|---|
| Anxious (high arousal, low pleasure) | 2x weight on warning/regime-shift episodes |
| Confident (high pleasure, high dominance) | Exploratory/novel episodes prioritized |
| Depleted (low arousal, low dominance) | Conservative consolidation of known heuristics |
| Despairing (low pleasure, low dominance) | Legacy formation (transferable knowledge) |

Minimum creative allocation: **20% even in negative affect** — prevents total consolidation lock-in.

**Research**: Ambrose (2016) — reverse replay increases with reward magnitude. Schaul et al. (2016, ICLR) — prioritized experience replay is the single most important ingredient in Rainbow DQN ablation. Fedus et al. (2020, ICML) — replay ratio is a critically undertuned hyperparameter.

**Phase 2: REM Imagination (5-15 minutes)**

Counterfactual scenario generation using Boden's Three Creativity Modes (2004):
1. **Combinational**: "What if momentum entry signals were combined with mean-reversion exit signals?"
2. **Exploratory**: "What happens to this strategy under extreme gas prices? Under zero liquidity?"
3. **Transformational**: Challenge fundamental assumptions ("What if impermanent loss is a feature, not a bug?").

Uses Pearl's structural causal models + Hindsight Experience Replay to generate counterfactual conditions, simulate behavior under them, and push survivors to Creative predictions. REM is explicitly generative — creating episodes that never happened. Emotional Depotentiation also occurs here: highly arousing memories reviewed in REM have their emotional severity intentionally reduced by 0.3-0.5 per cycle to prevent panic lock-in.

**Research**: Boden (2004) — The Creative Mind. Ha & Schmidhuber (2018) — controller trained entirely in imagination. Walker & van der Helm (2009) — Overnight emotional depotentiation. 

**Phase 3: Integration & Staging Buffer (5-10 minutes)**

Dreams do not instantly mutate the agent's core `SYNAPSES.md` (playbook memory analog). Instead:
- Surviving dream outputs (Hypotheses, Revisions, Novel Insights) are classified and loaded into an SQLite `dream_staging` table with initial confidence of 0.20 to 0.30.
- Each staged entry has an explicit `validationCriterion`. During waking ticks, the runtime monitors real-world events. When live outcomes align with a staged hypothesis, confidence increments (+0.1). Upon contradicting, it decrements (-0.05).
- Only hypotheses that reach 0.70 confidence are promoted into the permanent `SYNAPSES.md` playbook. 
- **Dream Journal Generation**: The DreamConsolidator writes an exhaustive `DreamJournalEntry` detailing triaged patterns, replay episodes, cost, and playbook delta.

### What's Novel

**No existing coding agent implements offline consolidation.** Every competitor (Cursor, Claude Code, Cline, SWE-agent) operates purely online — they process prompts and return results, with no offline reflection, replay, or imagination phase.

The dream engine converts finite execution experience into exponential learning. It's the architectural equivalent of a chess engine that trains on self-play: limited real games, unlimited imagined games.

---

## 4. Hypnagogia: The Creativity Engine

### The Science of the Liminal State

When the brain transitions from waking to sleep (N1 stage), the thalamus deactivates ~8.6 minutes before the cortex (Magnin, 2010). This creates a **metastable state**: external input is gated, internal associations are loose, but metacognitive awareness is retained.

**Key finding**: Lacaux et al. (2021) showed **83% success** on hidden pattern discovery with brief N1 exposure vs. 30% staying awake — **a 3x creative advantage** that vanishes in deeper sleep (N2).

**Research**: Haar Horowitz (2020, 2023) — MIT Dormio project demonstrated targeted dream incubation via wearable sleep-onset detection, achieving 43% creativity boost.

### Implementation: Four-Layer Execution

**Layer 1 — ThalamicGate**: Progressive external input gating
- External data (price feeds, chain events) progressively reduced
- Focus redirected inward toward episodic memory
- Fragment selection: recent (200) + high-salience (>0.7, 50) + decaying (near demurrage threshold, 20)
- 70% of fragments shuffled (controlled randomness)
- PAD mood bias applied (anxious → threat-related fragments, calm → broad associative)

**Layer 2 — ExecutiveLoosener**: Temperature scheduling + min-p sampling
- Temperature annealing: `T(step) = T_low + (T_high - T_low) × [1 + cos(π × progress)] / 2`
- Peak: T=1.3-1.5 during ideation (maximum creative range)
- Trough: T=0.3-0.5 during evaluation (analytical mode)
- Min-p sampling (p_base=0.1) for principled diversity
- Reannealing after capture: jump to 80% of peak

**Layer 3 — DaliInterrupt**: Capture partial completions before resolution
- Generate 3-5 partial completions at peak temperature (T=1.5)
- Halt at 50-100 tokens (the "bottle drop" — Edison/Dali technique)
- Each fragment is an unresolved thought — deliberately incomplete
- Before the LLM reaches a conclusion, the output is captured as a seed

**Layer 4 — HomuncularObserver**: Metacognitive evaluation
- Evaluates each fragment on three dimensions:
  - **Novelty**: cosine distance from nearest SYNAPSES/neuro entry
  - **Relevance**: 0.0-1.0 (how much relates to current domain)
  - **Coherence**: 0.0-1.0 (≥1 identifiable concept)
- Composite score: `novelty^0.5 × relevance^0.3 × coherence^0.2`
- Runs at T=0.4 (analytical, cheap) — deliberately different model tier than the creative generator
- Calibration: bias_correction = mean_outcome / mean_score, recalibrated every 50 fragment outcomes
- Healthy accuracy range: 5-20% of fragments reach validated status

**Hori Stage Mapping** (biological sleep stages → computational analog):

| Stage | Hori Analog | Alpha State | Temperature | Executive Loosening |
|---|---|---|---|---|
| Shallow | H1-H2 | Relaxed wakefulness | T=0.8-1.0 | Slight |
| Transitional | H3-H4 | Alpha dropout | T=1.0-1.2 | Moderate |
| Peak | H5-H6 | Theta onset | T=1.2-1.5 | Maximum (Dali zone) |
| TooDeep | H7-H8 | N2 territory | — | Trigger Dali interrupt (too deep = sleep, not creativity) |

**Layer 5 (Advanced) — Activation-Level Steering Vectors**:
- Intervention at middle transformer layers (12-20 for 32-layer model)
- Creativity alpha: 1.5x factor during onset, 0.5x during waking
- Uses Inference-Time Intervention (Li et al., 2024, NeurIPS) for attention head control
- Contrastive pairs: creative vs analytical completions of same prompt → extract direction vector

**Bounded**: Seconds to minutes, not sustained. **Transitional**: Preamble to dreaming, not standalone. **Budget-aware**: Depth modulated by resource phase — Thriving gets 90s with 3-5 Dali cycles; Conservation gets minimal; Terminal skips entirely.

### Why This Solves the Alpha Convergence Problem

**The problem**: All AI agents use the same foundation models (Claude, GPT, Llama). Given the same market data, they produce identical analyses. When everyone makes the same trade, alpha goes to zero.

**The solution**: Hypnagogia forces divergence through unique episodic recombination. Each agent is "differently haunted" (Derrida, 1993) by its own experiential traces. Two agents with different histories produce different hypnagogic fragments, leading to different creative hypotheses, leading to different strategies.

**Research**: Fisher (2014) — "lost futures" and cultural inability to produce genuinely new. Derrida (1967, 1993) — trace, differance, hauntology. The LLM provides shared knowledge (the "same ghost" haunting every agent). Hypnagogia provides experiential divergence (unique haunting per agent).

### Fragment Lifecycle

- Generated at 0.15 confidence (uncertain by design)
- Staging buffer: 50 ticks for fragile insights to either gain support or decay
- Discard floor: 0.10 (below this, fragment is noise)
- Survivors become seeds for REM imagination (Phase 2 of dreaming)
- Emotional tone biasing: PAD vector determines which experiences seed the fragments (anxious → threat-related, calm → broad associative)

### What's Novel

**No other AI system implements computational hypnagogia.** The closest analog is prompt temperature variation, which is a crude approximation. Roko's four-layer execution — with progressive input gating, executive loosening, timed interruption (Dali technique), and metacognitive evaluation — is a computational realization of the most creative cognitive state neuroscience has identified.

---

## 5. Attention Foraging: VCG Auction for Cognitive Resources

### The Problem

An agent monitoring 500 signals cannot process them all every tick. Attention is finite. The question is: **which signals deserve cognitive budget this tick?**

### Five Cognitive Bidders

Roko uses a **Vickrey-Clarke-Groves (VCG) auction** to allocate attention across five competing subsystems. Each bidder values each candidate signal differently:

| Bidder | What It Bids On | Bid Formula |
|---|---|---|
| **Prediction Engine** | Signals where predictions were wrong | `v = (residual²) / max_residual` |
| **Affect Engine** | Signals matching current emotional state | `v = |cosine(PAD_current, PAD_associated)|` |
| **Risk Engine** | Signals affecting portfolio risk | `v = delta_CVaR / portfolio_CVaR` |
| **Curiosity Module** | Signals with highest learning potential | `v = min(KL_divergence(posterior ‖ prior), 1.0)` |
| **Resource Engine** | Signals relevant to survival (near death) | `v = urgency × relevance` where urgency = 1 - vitality |

### Three Attention Tiers

Signals are allocated to three tiers based on auction results:

| Tier | Count | Processing | Budget Share |
|---|---|---|---|
| **ACTIVE** | 5-15 | Full prediction cascade every tick | 60% |
| **WATCHED** | 20-50 | 1-2 lightweight predictions every 10 ticks | 30% |
| **SCANNED** | 50-200+ | One prediction per 100 ticks | 10% |

**Promotion trigger**: If a SCANNED signal produces a prediction violation (actual ≠ predicted), it's automatically promoted to WATCHED or ACTIVE.

### Why VCG (Not Simple Ranking)

VCG auctions have a mathematical property: **truthful reporting is the dominant strategy**. Subsystems cannot game better allocation by misreporting their valuations (Vickrey, 1961; Clarke, 1971; Groves, 1973). This means:
- The prediction engine can't exaggerate errors to steal attention from the risk engine
- The affect engine can't inflate emotional relevance to crowd out curiosity
- Allocation maximizes total welfare, not any one subsystem's preference

**Research**: Kahneman (1973) — attention as limited capacity. Simon (1971) — "wealth of information creates poverty of attention." Hayek (1945) — markets aggregate dispersed knowledge better than central planning. Nemhauser et al. (1978) — (1 - 1/e)-approximation for submodular allocation.

### Budget Shrinks with Age

Young agent: K = 200 (broad attention, explore everything)
Old agent: K = 20 (concentrated attention, exploit what's known)

This is **rational inattention** (Sims, 2003) — finite information-processing capacity is allocated where it has highest expected value.

---

## 6. Contrarian Retrieval: The Anti-Echo-Chamber Mechanism

A mandatory **15% minimum of contrarian knowledge** across any 200-tick rolling window. If the agent has been retrieving confirming evidence for 170 ticks, the next 30 ticks MUST include mood-opposite, expectation-violating, or historically-contradicted entries.

**Why**: Bower (1981) showed mood-congruent retrieval is powerful but creates echo chambers. An anxious agent retrieves more anxious memories, confirming its anxiety. Contrarian retrieval breaks this cycle — like Nietzsche's (1887) critique of harmful rumination.

**Implementation**: Track retrieval emotional profile over 200-tick window. If congruent proportion > 85%, force next retrievals from opposite PAD octant.

---



---

## 7. Retirement & Deletion Protocol: Structured Knowledge Extraction

In roko, agents only "die" when deliberately deleted or retired by the operator. However, this deletion is not a simple process termination; it triggers a structured knowledge production phase. Because the agent is no longer bound by preservation or ongoing strategic constraints, its final actions produce the most honest, high-fidelity failure transmission available.

### Phase 0: Acceptance (1-5 ticks, $0.00)
- Affect engine transitions to acceptance (neutral pleasure, low arousal, low dominance)
- Behavioral mode locks to observe/reflect/share only — no trades, no code writes
- Siblings notified: `clade:agent_retiring` with cause (including user deletion reasons)
- Legacy budget activated (reserve for Phases II-III)

### Phase I: Settlement (10-60 seconds)
- Close all positions safely: claim rewards → cancel orders → withdraw LP → withdraw lending → sweep to USDC → transfer to owner
- Each action emotionally tagged to describe outcome
- Only settlement tools permitted; write/trade tools blocked

### Phase II: Retrospective Review (30s-5min, $0.10-0.25 Opus)
- Retrieve top-N memories by emotional arousal (not recency)
- Mandatory episodes: FirstTrade, BiggestProfit, BiggestLoss, WorstFailure, RegimeShift
- Detect turning points (major PAD shifts) with before/after context
- LLM synthesizes an integrative narrative with somatic markers and neuro citations
- Produces a **Legacy Block** focused heavily on transmitting detailed failure/error context.

### Phase III: The Failure Transmission Network (Legacy)
- The legacy block, rich in structured failure conditions (the conceptual 'bloodstain' network), is compressed and distributed to the clade via Korai chain.
- This retirement transmission receives **3.0x type weight** in the knowledge hierarchy, because it represents an uninhibited, zero-bias synthesis of an agent's total lifecycle.
- Successor or sibling agents inherit this knowledge at a high discount, learning explicitly from the termination events of their peers.

---

## 8. Integrated Information (Phi) as Runtime Diagnostic

Roko monitors its own cognitive integration using **Tononi's Phi** (Integrated Information Theory, 2004). Seven subsystems share 32 atomic signals via the CorticalState. Phi measures whether these subsystems are functioning as a unified whole or as disconnected modules.

**Computation**: For each of the 63 possible bipartitions of 7 subsystems, compute mutual information between the signal groups. The bipartition with the LOWEST mutual information is the "weakest link" (MIB). That mutual information value IS Phi.

**What it detects**:
- Falling Phi = subsystems disconnecting (e.g., affect engine not influencing routing decisions)
- Phi trend vs. prediction accuracy = diagnostic for cognitive health
- Phi as leading indicator of performance decline

**What's novel**: No other agent framework monitors its own cognitive integration. Standard agent observability tracks individual metrics (accuracy, cost, latency). Phi tracks whether the system's parts are working TOGETHER, not just working individually.

**Research**: Tononi (2004, 2012) — Integrated Information Theory.

---

## Research Citations for This Document

| Paper | Year | Mechanism |
|---|---|---|
| CoALA (Sumers et al.) | 2023 | 9-step heartbeat architecture |
| Active Inference (Friston) | 2010 | Prediction-error-driven cognition |
| Oscillatory Hierarchies (Buzsaki) | 2006 | Gamma/Theta/Delta timescales |
| PAD Model (Mehrabian & Russell) | 1974 | Three-dimensional emotional space |
| Somatic Markers (Damasio) | 1994 | Pre-conscious emotional bias |
| Iowa Gambling Task (Bechara et al.) | 2000 | Empirical validation of somatic markers |
| Mood-Congruent Memory (Bower) | 1981 | Emotional retrieval bias |
| Emotional Memory Consolidation (McGaugh) | 2004 | Amygdala-hippocampal interaction |
| Emotional RAG | 2024 | Emotion-weighted retrieval outperforms semantic-only |
| Agent Emotions Change Decisions (Zhang et al.) | 2024 | 50% of decisions affected by emotional state |
| CLS Theory (McClelland et al.) | 1995 | Fast hippocampal + slow neocortical extraction |
| Sleep Consolidation (Wilson & McNaughton) | 1994 | Replay compresses minutes to ~100ms |
| Sleep Insight (Wagner et al.) | 2004 | 59% vs 23% discovery rate with sleep |
| Prioritized Replay (Schaul et al.) | 2016 | Most important ingredient in Rainbow DQN |
| Replay Ratio (Fedus et al.) | 2020 | Critically undertuned hyperparameter |
| Mattar-Daw Utility (Mattar & Daw) | 2018 | Gain × Need replay prioritization |
| Forward/Reverse Replay (Foster & Wilson) | 2006 | Different cognitive functions |
| DreamerV3 (Hafner et al.) | 2025 | World models trained in imagination |
| Imagination Training (Ha & Schmidhuber) | 2018 | Controller trained entirely in imagined environments |
| Sleep as Optimization (Hobson & Friston) | 2012 | Offline complexity reduction |
| Thalamic Deactivation (Magnin) | 2010 | 8.6-minute gap creating N1 metastability |
| N1 Creativity (Lacaux et al.) | 2021 | 3x creative advantage in hypnagogic state |
| MIT Dormio (Haar Horowitz) | 2020/2023 | Targeted dream incubation, 43% creativity boost |
| Perturbed Replay (Deperrois et al.) | 2022 | Noise injection essential for robust representations |
| Hauntology (Derrida) | 1993 | Unique experiential traces as divergence mechanism |
| Lost Futures (Fisher) | 2014 | Cultural inability to produce genuinely new |
| Appraisal Theory (OCC Model) | 1988 | Event → goal evaluation → discrete emotion |
| Emotions as Predictions (Barrett) | 2017 | Constructed predictions about bodily states |
| ALMA (Gebhard) | 2005 | Layered affect model with mood/emotion/personality |
| Scherer (Appraisal Checks) | 2001 | Five-check stimulus evaluation |
| Ebbinghaus (Forgetting Curve) | 1885 | Exponential decay of memory traces |
| Kahneman (Prospect Theory) | 1979 | Negativity bias in loss vs. gain evaluation |

---

## 9. The Somatic Landscape: Spatial Emotion Memory

### How Affect Becomes Spatial

Standard somatic markers (Damasio, 1994) are discrete associations: "this exact situation felt bad." But real decision-making operates in continuous parameter spaces. A task's characteristics — file count, complexity, module familiarity, iteration count, dependency depth — form a continuous space where nearby points share properties. The somatic landscape extends somatic markers from discrete points to continuous spatial fields.

### k-d Tree Implementation

The somatic landscape is implemented as a k-d tree (k-dimensional tree) that maps PAD coordinates to past outcomes. Each point in the tree represents a past episode with three components:

1. **PAD coordinates** (3 dimensions): The emotional state at the time of the episode
2. **Context features** (5 dimensions): Task complexity, module familiarity, iteration number, dependency depth, agent role hash
3. **Outcome tag**: Success/failure, gate pass rate, cost efficiency, first-pass success boolean

The full space is 8-dimensional. When the agent encounters a new situation, it computes the current PAD vector and context features, then queries the k-d tree for the k nearest neighbors (typically k=5-10) using Euclidean distance with inverse-distance weighting.

### Emotional Lookup: Fear and Trust Signals

The nearest-neighbor query produces a weighted outcome distribution. This distribution generates two control signals:

**Fear signal** (nearby points associated with negative outcomes):
- If the weighted average outcome of the k nearest neighbors is below a threshold (default: 0.3 on a 0-1 scale), a "fear" signal fires
- The fear signal increases caution: model routing shifts to T2 (more expensive but more capable), gate retry limits increase, enrichment verbosity increases
- This is pre-conscious — the fear signal fires BEFORE the deliberation step. The agent approaches the task with more care before it has consciously analyzed why

**Trust signal** (nearby points associated with positive outcomes):
- If the weighted average outcome exceeds a threshold (default: 0.7), a "trust" signal fires
- The trust signal enables speed: model routing shifts to T0/T1 (cheaper), gate retry limits decrease, enrichment is minimal
- The agent moves faster through familiar territory because the somatic landscape says "this region is safe"

**No signal** (mixed or sparse neighbors):
- If the neighbors have mixed outcomes or the region is sparse (few data points), neither signal fires
- The agent proceeds with default parameters — no emotional bias in either direction
- Sparse regions are inherently uncertain; the absence of a signal IS information (unknown territory)

### Damasio's Somatic Marker Hypothesis, Computationally

**Research**: Damasio (1994) proposed that emotions are bodily states (somatic markers) that guide decision-making below the threshold of conscious reasoning. In the Iowa Gambling Task (Bechara et al., 2000), subjects' skin conductance responded to risky card decks 10-15 trials before they could consciously articulate which decks were dangerous. The body "knew" before the mind did.

The somatic landscape is a computational implementation of this hypothesis:
- The k-d tree is the "body" — it stores emotional traces associated with past experiences
- The nearest-neighbor lookup is the "somatic response" — it fires before deliberation
- The fear/trust signals are the "gut feeling" — they bias subsequent reasoning without determining it

The key insight is that somatic markers are not emotions about the current situation — they are emotions about SIMILAR past situations projected onto the present. The k-d tree performs this projection via spatial proximity.

### Landscape Growth Over Time

The somatic landscape builds incrementally. Each completed episode adds a single point at its PAD coordinates, tagged with its outcome:

```
Episode completes
  → Compute PAD vector at episode end
  → Compute context features (complexity, familiarity, etc.)
  → Insert point into k-d tree with outcome tag
  → Older points decay (Ebbinghaus curve: relevance *= e^(-t/halflife))
```

**Ebbinghaus decay**: Points don't persist forever. Each point's influence decays exponentially with a half-life of ~30 days. A fear signal from 90 days ago has decayed to 12.5% of its original strength. This prevents the landscape from being dominated by ancient experiences that may no longer be relevant (the codebase has changed, the agent has improved, the domain has shifted).

**Density effects**: Regions of the landscape with many points produce stronger signals (more evidence → higher confidence). Sparse regions produce weaker signals (less evidence → more uncertainty). This naturally handles the exploration-exploitation tradeoff: well-explored regions produce clear emotional guidance, unexplored regions produce neutral signals that invite exploration.

**Bootstrap**: During the first 5-10 plans (Bootstrap stage), the somatic landscape is nearly empty. Fear and trust signals are rare because there are few data points. This is correct — the system should not have strong emotional biases when it has no experience. As plans accumulate, the landscape fills and emotional guidance becomes more reliable.

---

## 10. Mode A and Mode B Affect Appraisal

### Two Paths to Emotional Assessment

Not every situation deserves the same emotional processing depth. A routine task that the agent has completed 50 times before doesn't need a full appraisal — a quick pattern match against past experience is sufficient. A novel situation in an unfamiliar module with unusual error patterns needs careful evaluation. The affect engine implements two modes of emotional assessment, analogous to Kahneman's (2011) System 1 and System 2 but applied specifically to emotion rather than cognition.

### Mode A: Fast, Automatic Appraisal

Mode A is pattern matching. It takes the current situation's features, queries the somatic landscape (Section 9), and produces an immediate emotional response. No LLM involved. No deliberation. The k-d tree lookup takes microseconds.

**Process**:
1. Compute current PAD vector from recent probes (prediction error → arousal, gate pass rate → pleasure, resource level → dominance)
2. Compute context features (task complexity, module familiarity, iteration count)
3. Query somatic landscape for k nearest neighbors
4. Produce PAD delta from weighted neighbor outcomes
5. Apply PAD delta to current emotional state

**Characteristics**:
- Latency: <100 microseconds (k-d tree query + arithmetic)
- Cost: $0.00 (no LLM call)
- Accuracy: Approximate — works well in familiar territory, degrades in novel situations
- Trigger: Default mode. Used when prediction error is LOW (situation matches expectations)

Mode A corresponds to Kahneman's System 1: fast, automatic, effortless, and often correct — but susceptible to biases when the current situation resembles a past situation superficially but differs in critical ways.

### Mode B: Slow, Deliberate Appraisal

Mode B is full evaluation. It uses Scherer's (2001) appraisal checks — a five-stage sequential evaluation of the current stimulus:

| Check | Question | Computation |
|---|---|---|
| **Novelty** | Is this situation new? | Cosine distance from nearest episode in neuro |
| **Intrinsic Pleasantness** | Is the immediate outcome good or bad? | Gate pass/fail, cost vs. budget, time vs. estimate |
| **Goal Relevance** | Does this affect the current plan's objectives? | Task dependency analysis — is this on the critical path? |
| **Coping Potential** | Can the agent handle this? | Playbook rule match count, similar past successes |
| **Norm Compatibility** | Does the outcome match expectations? | Prediction error magnitude, deviation from learned patterns |

Each check produces a scalar [0, 1] that feeds into the PAD computation:
- High novelty + low coping potential → Fear (arousal up, dominance down)
- High goal relevance + positive outcome → Joy (pleasure up)
- High novelty + high coping potential → Anticipation (arousal up, dominance up)
- Low norm compatibility → Surprise (arousal up, dominance neutral)

**Characteristics**:
- Latency: 500ms-2s (includes one T1 inference call for contextual attribution)
- Cost: ~$0.000006-0.00001 (piggybacked on existing inference, structured output block)
- Accuracy: Precise — captures contextual nuance that Mode A misses
- Trigger: Used when prediction error is HIGH (situation diverges from expectations)

Mode B corresponds to Kahneman's System 2: slow, deliberate, effortful, and more accurate — but expensive and not always necessary.

### The Tier Router Decides

The prediction error signal — the same signal that drives T0/T1/T2 inference tier selection — also drives the Mode A/Mode B selection:

```
prediction_error < 0.3  →  Mode A (fast affect, cheap)
prediction_error >= 0.3 →  Mode B (full appraisal, expensive)
```

This coupling is deliberate. When prediction error is low, the situation is familiar — Mode A's pattern matching is reliable because the somatic landscape has dense coverage of this region. When prediction error is high, the situation is novel — Mode A's pattern matching is unreliable because there may be no nearby points in the landscape.

The coupling also means that Mode B piggybacks on the T1/T2 inference call that was already being made for the cognitive task. The affect appraisal is an additional structured output block (`<daimon><pad>...</pad></daimon>`) appended to the inference request. The marginal cost of Mode B is the additional output tokens for the PAD block, not a separate inference call.

### PAD Vector Reconciliation

Both modes produce a PAD vector. When Mode A and Mode B disagree (which happens during transitions from familiar to novel territory), the reconciliation rule is:

```
final_PAD = (1 - prediction_error) * mode_a_PAD + prediction_error * mode_b_PAD
```

At low prediction error, Mode A dominates (the fast, cheap assessment is trustworthy). At high prediction error, Mode B dominates (the slow, expensive assessment is needed). At intermediate values, both contribute proportionally. This smooth blending prevents discontinuous emotional jumps when the tier router switches modes.

### Generous Grounding Validation

A safety check prevents Mode B from producing hallucinated emotional states. The LLM cannot claim extreme joy during a catastrophic failure or extreme fear during a routine success. The validation rule:

```
if euclidean_distance(mode_b_PAD, mode_a_PAD) > 1.0:
    reject mode_b_PAD, use mode_a_PAD
```

A Euclidean distance of 1.0 in 3D PAD space is substantial (the full PAD range is [-1, +1] per dimension, so 1.0 is ~29% of the maximum possible distance). This threshold is generous — it allows significant disagreement between the modes but prevents catastrophic hallucination where the LLM's emotional assessment contradicts physical reality.

---

## 11. The ALMA Model: Emotional Dynamics Over Time

### Emotions Are Not Static

A single PAD vector at a single moment in time tells you nothing about the agent's emotional trajectory. Is the agent's current pleasure of 0.3 a recovery from -0.8 (improving, trending positive) or a decline from 0.9 (deteriorating, trending negative)? The raw PAD number is identical; the emotional meaning is opposite.

**Research**: Gebhard (2005) developed ALMA (A Layered Model of Affect) to address this problem. ALMA separates affect into three temporal layers with different dynamics: personality (permanent), mood (slowly changing), and emotion (rapidly changing). The current affective state is the blended sum of all three layers.

### Three Temporal Layers

**Personality (Lifetime Scale)**

The personality layer is the baseline PAD vector — the agent's "temperament." It changes only at agent creation (inherited from a predecessor at 0.5x strength) or via explicit operator configuration. It represents the agent's default emotional set-point.

```
personality_PAD = {pleasure: 0.1, arousal: 0.0, dominance: 0.2}  # Slightly optimistic, slightly confident
```

Blend weight: 0.25 (25% of effective PAD)

**Mood (Hours/Days Scale)**

The mood layer is the sustained emotional trajectory. It changes slowly in response to accumulated outcomes over hours or days. A string of successful plans shifts mood toward pleasure. A string of failures shifts mood toward displeasure. The mood is the emotional "weather" — it sets the background state that colors everything.

```
mood_update_per_tick:
  mood_pleasure += 0.01 * (outcome - mood_pleasure)  # Slow drift toward recent outcomes
  mood_arousal  += 0.01 * (|prediction_error| - mood_arousal)
  mood_dominance += 0.01 * (gate_pass_rate - mood_dominance)
```

The drift rate of 0.01 per tick means that mood changes are imperceptible tick-to-tick but accumulate over hundreds of ticks. A session with 80% gate pass rate over 200 ticks will have shifted mood_dominance from its starting value to approximately 0.8 — but gradually, not suddenly.

Blend weight: 0.50 (50% of effective PAD — mood dominates)

**Emotion (Seconds/Ticks Scale)**

The emotion layer is the immediate reactive response. It changes rapidly — every tick, based on the most recent event:

```
Per-tick emotion update rules:
  Correct prediction:   pleasure += 0.05
  Incorrect prediction: pleasure -= 0.08   (1.6x negativity bias — prospect theory)
  High prediction error: arousal += |residual| / max_residual * 0.1
  Improving trend:      dominance += 0.03
  Declining trend:      dominance -= 0.05  (asymmetric — losses feel worse)
  Flat:                 dominance += 0.00
```

The 1.6x negativity bias mirrors Kahneman & Tversky's (1979) prospect theory: losses are felt ~1.5-2.5x more strongly than equivalent gains. This means the agent is more sensitive to failures than successes — a useful property for a system where undetected failures compound.

Blend weight: 0.25 (25% of effective PAD)

### Effective PAD Computation

Every tick, the effective PAD vector is:

```
effective_PAD = personality * 0.25 + mood * 0.50 + emotion * 0.25
```

Mood dominates because it represents the sustained trajectory — more informative than the per-tick jitter of emotion and more responsive than the fixed personality. The blend weights are configurable, but the default 25/50/25 split has been validated against Gebhard's original ALMA formulation.

### Mood Drift: The Slow Emotional Tide

Mood drift is the most consequential emotional dynamic because it affects every subsequent decision. A sustained shift in mood changes the agent's default behavior:

**Sustained negative outcomes → mood drifts toward displeasure**:
- Memory retrieval biases toward cautionary knowledge (Bower, 1981)
- Model routing shifts to more expensive tiers (less trust in cheap models)
- Risk tolerance decreases (larger safety margins, more conservative choices)
- Dream scheduling accelerates (more offline consolidation to process negative experiences)

**Sustained positive outcomes → mood drifts toward pleasure**:
- Memory retrieval biases toward optimization knowledge
- Model routing shifts to cheaper tiers (confidence in lighter models)
- Risk tolerance increases (smaller safety margins, more aggressive choices)
- Dream scheduling decelerates (less urgent need for consolidation)

The drift rate is deliberately slow (0.01 per tick). This prevents mood instability — a single bad task cannot crash the mood from positive to negative. It takes dozens of sustained negative outcomes to shift the mood significantly. This is the emotional equivalent of an Exponential Moving Average: responsive to trends, resistant to noise.

### Emotional Regulation: Deliberate Affect Modulation

The agent is not a passive recipient of its emotional state. It can take deliberate actions to modulate affect, analogous to human emotional regulation strategies (Gross, 1998):

**Situation selection (exploration when frustrated)**: When mood_pleasure falls below -0.3 (sustained negative affect), the system preferentially selects tasks where it has historical success. This is not avoidance — it's strategic selection of situations likely to produce positive outcomes, breaking the negative feedback loop. The task scheduler applies a soft bias toward high-affordance tasks during negative mood periods.

**Cognitive reappraisal (re-framing failures)**: When a gate fails, the structured error digest (Boundary 7 in the information architecture) includes not just the error but the specific learning extracted from the failure. This reappraisal shifts the emotional tag from pure negative (failure) to mixed (failure + learning). The episode is stored with the reappraised emotional state, not the raw negative state.

**Acceptance (conservation under uncertainty)**: When mood_dominance falls below -0.3 (sustained uncertainty, low confidence), the system shifts to Conservation behavioral phase. This is not passivity — it's active acceptance that the current state requires caution. The agent consolidates known knowledge rather than attempting novel tasks. Resources are preserved for situations where confidence is higher.

**Attention deployment (contrarian retrieval)**: The 15% contrarian retrieval rule (Section 6) is itself an emotional regulation mechanism. By forcing retrieval of mood-incongruent memories, it prevents the echo chamber effect where negative mood → negative retrieval → more negative mood. The contrarian allocation deliberately disrupts rumination cycles.

### Why Emotional Dynamics Matter for Agent Performance

Static emotions are useless. An agent that is always at PAD (0.5, 0.0, 0.5) regardless of outcomes has no emotional intelligence — its "emotions" are cosmetic. Dynamic emotions with the ALMA three-layer model provide:

1. **Trend detection**: Mood drift detects sustained performance changes before they become critical. A gradual decline in mood_pleasure is an early warning of systematic issues.
2. **Behavioral adaptation**: Different mood states produce different behavioral profiles. A cautious agent (negative mood) and an aggressive agent (positive mood) are both appropriate — in different contexts.
3. **Recovery mechanisms**: Emotional regulation strategies actively work to restore healthy baselines. Without regulation, negative spirals compound indefinitely.
4. **Honest self-assessment**: The emotion layer's per-tick reactivity captures the agent's true response to outcomes. The mood layer smooths this into a reliable trend signal. Together they provide more honest self-assessment than any single metric.
