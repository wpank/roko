# Dual-Process Cognition and the T0/T1/T2 Tiers

> Roko's three-tier model for fast reflexive thinking vs. slow deliberate thinking —
> the cognitive equivalent of System 1 and System 2.

**Status**: Shipping
**Crate**: `roko-agent`
**Depends on**: [Three Cognitive Speeds](../07-speeds/README.md),
[ROUTE stage](03-stage-route.md), [loop\_tick()](09-loop-tick-code.md)
**Used by**: [Speed Coordination](../07-speeds/04-speed-coordination.md)
**Last reviewed**: 2026-04-19

---

## TL;DR

Kahneman's System 1 (fast, automatic) and System 2 (slow, deliberate) map directly
onto Roko's T0/T1/T2 speed tiers. T0 (Gamma, 5–15 s period) is fast-path processing
with minimal context and cheap models. T1 (Theta, ~75 s) is reflective processing with
rich context and more capable models. T2 (Delta, hours) is offline consolidation — no
real-time execution, only memory reorganization. The loop's ROUTE stage is the gating
mechanism: low-confidence routes escalate from T0 to T1 to T2.

---

## The Idea

Human cognition works on at least two speeds. Kahneman called them System 1 (fast,
associative, automatic, cheap) and System 2 (slow, logical, deliberate, expensive).
Neuroscience maps these roughly onto different oscillatory regimes: fast gamma
oscillations (30–80 Hz) support reactive processing; slower theta oscillations (4–8 Hz)
support working memory and deliberation; delta waves (0.5–4 Hz) dominate deep offline
consolidation.

Roko's three speed tiers (Gamma / Theta / Delta) are a computational instantiation of
the same hierarchy. The key insight is that **you don't always need System 2.** Most
stimuli are routine; the agent should handle them quickly and cheaply. Only genuinely
novel or high-stakes situations justify the cost of deliberate processing.

The decision about which tier to use is made by the ROUTE stage based on routing
confidence. This is not an approximation — it is the correct architectural location for
the System 1 / System 2 decision, because routing confidence is the most direct
measure of whether the agent "knows what to do."

---

## The Three Tiers

### T0 — Gamma (System 1)

- **Trigger**: routing confidence ≥ 0.85
- **Speed**: 5–15 s per tick period (tick itself is typically < 2 s)
- **Context window**: 4 096 tokens
- **Model**: fastest available (e.g., GPT-4o Mini, Claude Haiku)
- **Substrate lookback**: last 60 s
- **Candidate cap**: 16 Engrams
- **Characteristic**: pattern-matching, reflexive, habits-as-routes

T0 is the steady-state operating mode for well-understood tasks. An agent answering
routine user queries, monitoring a data stream, or executing a familiar sub-task runs
almost entirely at T0. The loop completes in under 2 s wall clock; most of that is
the model API call.

### T1 — Theta (System 2)

- **Trigger**: routing confidence 0.60–0.85, OR escalation from T0 due to VERIFY failure
- **Speed**: ~75 s per tick period (tick itself may take 15–60 s)
- **Context window**: 16 384 tokens
- **Model**: more capable (e.g., GPT-4o, Claude Sonnet)
- **Substrate lookback**: last 10 min
- **Candidate cap**: 64 Engrams
- **Characteristic**: deliberate reasoning, cross-referencing, chain-of-thought

T1 activates when the agent encounters an unfamiliar stimulus, a novel routing
decision, or a previously-failed verification. It assembles a richer context, uses a
more capable model, and takes more time. The extra investment typically results in
higher-quality output and fewer VERIFY failures.

### T2 — Delta (System 3 / offline)

- **Trigger**: routing confidence < 0.60, OR scheduled consolidation cycle
- **Speed**: hours between cycles
- **Context window**: 128 000 tokens (or full substrate)
- **Model**: most capable, or no model (pure memory reorganization)
- **Substrate lookback**: up to 24 h
- **Characteristic**: offline consolidation, knowledge reorganization, no live execution

T2 does not produce real-time output. It reorganizes knowledge in the Substrate,
promotes frequently-used Engrams to higher-durability tiers, prunes stale knowledge,
and updates routing priors. The Dreams cross-cut (offline learning) runs primarily at
T2.

---

## The Escalation Ladder

```
Incoming stimulus
       │
       ▼
   T0 attempt
   confidence ≥ 0.85? ──YES──► execute at Gamma speed
       │
       NO
       ▼
   T1 attempt
   confidence ≥ 0.60? ──YES──► execute at Theta speed
       │
       NO
       ▼
   T2 deferral
   (or route.uncertain Pulse → human / sub-agent)
```

Escalation is one-way per stimulus. An attempt that starts at T0 and fails VERIFY
does not retry at T0; it escalates to T1. If T1 also fails VERIFY, it escalates to T2
(deferral or consolidation).

---

## Interaction with Active Inference

The dual-process model interacts with active inference at the escalation boundary.
When a T0 attempt produces a high prediction error (via the `predict.error` Pulse),
the active inference layer increases the routing uncertainty prior for similar stimuli.
Over time, previously T0-routed tasks may shift to T1 if the model repeatedly
misjudges them.

See [Active Inference](11-active-inference.md) for the full predict/publish/correct
cycle.

---

## Configuration

```toml
# roko-agent config
[dual_process]
t0_confidence_threshold = 0.85   # above this → T0 (Gamma)
t1_confidence_threshold = 0.60   # above this → T1 (Theta)
# below t1_confidence_threshold → T2 deferral

[gamma]
period_secs         = 10
max_context_tokens  = 4096
candidate_cap       = 16
substrate_lookback  = "60s"

[theta]
period_secs         = 75
max_context_tokens  = 16384
candidate_cap       = 64
substrate_lookback  = "10m"
```

---

## See also

- [Three Cognitive Speeds](../07-speeds/README.md) — the full speed system
- [ROUTE stage](03-stage-route.md) — where the T0/T1/T2 decision is made
- [Active Inference](11-active-inference.md) — how prediction errors update routing priors
- [Neuro cross-cut](../09-cross-cuts/01-neuro.md) — Neuro's role in T2 consolidation
- [Dreams cross-cut](../09-cross-cuts/03-dreams.md) — offline learning at T2
