# Gamma — The Reactive Speed Tier

> Fast, cheap, automatic processing for stimuli the agent already knows how to handle.

**Status**: Shipping
**Crate**: `roko-agent`
**Named after**: Gamma oscillations (30–80 Hz) in the EEG — the fastest cortical rhythm,
associated with local feature binding and rapid sensorimotor integration.
**Last reviewed**: 2026-04-19

---

## TL;DR

Gamma is the default operating speed for a Roko agent. It handles stimuli where
routing confidence is ≥ 0.85 — situations the agent has successfully processed before.
Context window is 4 096 tokens; model is the fastest available; substrate lookback is
60 s. A Gamma tick typically completes in under 2 s wall clock.

---

## Parameters

| Parameter | Value | Configurable? |
|---|---|---|
| Routing confidence threshold | ≥ 0.85 | Yes |
| Nominal tick period | 10 s | Yes (5–15 s range) |
| Max context tokens | 4 096 | Yes |
| Substrate lookback | 60 s | Yes |
| Candidate cap (QUERY) | 16 | Yes |
| Recommended model tier | Fast / cheap (e.g., GPT-4o Mini, Claude Haiku) | Yes |
| Cost per tick (typical) | < $0.001 | Depends on model |
| Wall time (ex-ACT) | < 20 ms | — |
| Wall time (inc-ACT, typical) | < 2 s | Depends on model |

---

## What Makes a Tick "Gamma"

A tick runs at Gamma speed when the ROUTE stage produces a `RouteDecision` with
`confidence ≥ gamma_threshold` (default 0.85). This means:

- The agent has seen similar stimuli many times before.
- The routing prior (maintained by CascadeRouter's Wilson CI) has a tight confidence
  interval around the best target.
- The expected verification pass rate for this target/stimulus pair is high.

In practice, the majority of ticks for a well-trained agent in a stable environment
are Gamma ticks. An agent that primarily answers questions about a narrow domain will
have routing confidence > 0.90 for 80–90% of stimuli.

---

## What Gamma Does Well

- **Reactive monitoring** — checking a data stream, alerting on threshold crossings,
  responding to simple user queries.
- **Execution of known sub-tasks** — writing a file, calling a known API, extracting
  a value from a structured document.
- **Pattern-matched responses** — questions where the answer is essentially a template
  instantiation from prior knowledge.

---

## What Gamma Does Poorly

- **Novel questions** — when there is no prior routing history, confidence is low and
  Gamma cannot activate.
- **Multi-step reasoning** — the 4 096-token context cannot hold a long chain of
  reasoning steps.
- **Conflict resolution** — when the composed context contains conflicting evidence,
  a Gamma model often picks whichever appears first.

When Gamma fails (VERIFY returns SoftFail or HardFail), the tick is retried at Theta.
This is automatic — the operator does not need to configure it explicitly.

---

## Configuration

```toml
# roko-agent config
[gamma]
period_secs          = 10       # tick target period
min_period_secs      = 5        # floor (under load)
max_period_secs      = 15       # ceiling (when idle)
max_context_tokens   = 4096
candidate_cap        = 16
substrate_lookback   = "60s"
confidence_threshold = 0.85     # minimum confidence to use Gamma
```

---

## Adaptive Period

The Gamma period is not fixed — it adapts based on:

- **Agent load**: under high Pulse throughput, the period shortens toward `min_period_secs`.
- **Idle state**: if no Pulses arrive, the period extends toward `max_period_secs` to
  avoid wasting compute on empty ticks.
- **StuckDetector**: if stuck is detected, the period is temporarily shortened to 1 s
  to trigger rapid recovery attempts.

See [Speed Coordination](04-speed-coordination.md) for the full adaptive clock logic.

---

## Observability

| Metric | Description |
|---|---|
| `gamma.tick_rate` | Ticks per second running at Gamma speed |
| `gamma.confidence_avg` | Rolling average routing confidence for Gamma ticks |
| `gamma.verify_pass_rate` | Fraction of Gamma ticks that pass VERIFY |
| `gamma.escalation_rate` | Fraction of Gamma ticks that escalate to Theta |

A healthy agent has `gamma.verify_pass_rate > 0.90` and `gamma.escalation_rate < 0.15`.

---

## See also

- [Overview](00-overview.md) — the three-speed system
- [Theta (Reflective)](02-theta-reflective.md) — the speed Gamma escalates to
- [Dual-Process](../06-loop/10-dual-process.md) — the confidence threshold decision
- [Triggers](05-triggers.md) — what causes Gamma to activate or escalate
