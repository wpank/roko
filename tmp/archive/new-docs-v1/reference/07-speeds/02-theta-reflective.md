# Theta — The Reflective Speed Tier

> Slower, more deliberate processing for stimuli the agent has not fully mastered.

**Status**: Shipping
**Crate**: `roko-agent`
**Named after**: Theta oscillations (4–8 Hz) in the EEG — associated with working
memory, spatial navigation, and the coordination of distant brain regions.
**Last reviewed**: 2026-04-19

---

## TL;DR

Theta activates when routing confidence is between 0.60 and 0.85, or when a prior
Gamma tick failed VERIFY. It uses a larger context window (16 384 tokens), a more
capable model, and looks back 10 minutes in the substrate. A Theta tick typically
takes 5–60 s depending on the model and context size. Cost is ~10–100× Gamma.

---

## Parameters

| Parameter | Value | Configurable? |
|---|---|---|
| Routing confidence range | 0.60–0.85 | Yes |
| Nominal tick period | 75 s | Yes (30–180 s range) |
| Max context tokens | 16 384 | Yes |
| Substrate lookback | 10 min | Yes |
| Candidate cap (QUERY) | 64 | Yes |
| Recommended model tier | Capable (e.g., GPT-4o, Claude Sonnet) | Yes |
| Cost per tick (typical) | $0.01–$0.10 | Depends on model |
| Wall time (inc-ACT, typical) | 5–60 s | Depends on model |

---

## What Makes a Tick "Theta"

A tick runs at Theta when:

1. The ROUTE stage produces confidence 0.60–0.85 on the first attempt.
2. A prior Gamma tick on the same stimulus returned a SoftFail or HardFail from VERIFY.
3. The StuckDetector forced a tier escalation.

Theta is the agent's "thinking hard" mode. It is not a failure state — it is the
appropriate response to genuinely difficult stimuli. An agent that *never* runs Theta
ticks either handles only trivial tasks or has never been tested on hard ones.

---

## The Theta Difference from Gamma

| Dimension | Gamma | Theta |
|---|---|---|
| Context window | 4 096 tokens | 16 384 tokens |
| Candidates composed | 3–5 | 12–20 |
| Substrate lookback | 60 s | 10 min |
| Model | Fast / cheap | More capable |
| Chain-of-thought | No (token budget too small) | Yes (scaffold added by Composer) |
| Hallucination check | Standard | Stricter threshold |
| Cost | < $0.001 | $0.01–$0.10 |

The 4× larger context window allows the Composer to include:
- A full chain-of-thought scaffold
- Contradictory prior evidence (letting the model reason through the conflict)
- Multiple prior answers to related questions for comparison
- Relevant provenance information

---

## Chain-of-Thought Scaffold

At Theta, the Composer automatically adds a chain-of-thought scaffold to the system
prompt:

```
Think step by step. Before providing your answer:
1. List the relevant facts from the context.
2. Identify any contradictions or uncertainties.
3. Reason through the most likely answer.
4. State your confidence level.
Then provide your final answer.
```

This scaffold adds ~80 tokens to the prompt but consistently improves VERIFY pass
rates on complex questions.

---

## Escalation from Gamma

The escalation path from a failed Gamma tick:

1. Gamma VERIFY fails (HardFail or SoftFail).
2. REACT schedules next tick with `outcome_modifier = 0.5` (sooner) and forces T1.
3. Next tick runs at Theta with:
   - Same stimulus
   - Wider QUERY (lookback extended, candidate cap 64)
   - Richer COMPOSE
   - More capable model

If the Theta tick also fails VERIFY, the agent escalates to T2 (Delta deferral or
Delta consolidation).

---

## Theta and Neuro

The Neuro cross-cut has enhanced participation in Theta ticks. When the agent runs at
Theta, Neuro may:

- Perform a "deep retrieval" from the HDC index using a broader similarity threshold
- Surface related semantic clusters, not just directly matching Engrams
- Include recently-written research notes from prior research agent runs

This is configured in Neuro's speed-tier policy. See
[Neuro cross-cut](../09-cross-cuts/01-neuro.md).

---

## Configuration

```toml
[theta]
period_secs             = 75
min_period_secs         = 30
max_period_secs         = 180
max_context_tokens      = 16384
candidate_cap           = 64
substrate_lookback      = "10m"
confidence_threshold_lo = 0.60   # below this → defer / Delta
confidence_threshold_hi = 0.85   # above this → Gamma
chain_of_thought        = true   # inject CoT scaffold in system prompt
```

---

## Observability

| Metric | Description |
|---|---|
| `theta.tick_rate` | Ticks per second at Theta speed |
| `theta.verify_pass_rate` | Fraction of Theta ticks passing VERIFY |
| `theta.escalation_rate` | Fraction of Theta ticks escalating to Delta |
| `theta.cost_avg` | Average cost per Theta tick |

A healthy agent has `theta.verify_pass_rate > 0.85`. If `theta.escalation_rate > 0.20`,
the agent is encountering too many stimuli it cannot handle — consider adding domain
knowledge or adjusting the confidence threshold.

---

## See also

- [Overview](00-overview.md) — the three-speed system
- [Gamma (Reactive)](01-gamma-reactive.md) — the speed that escalates to Theta
- [Delta (Consolidation)](03-delta-consolidation.md) — the speed Theta escalates to
- [Dual-Process](../06-loop/10-dual-process.md) — the confidence band that defines Theta
- [Neuro cross-cut](../09-cross-cuts/01-neuro.md) — enhanced retrieval at Theta speed
