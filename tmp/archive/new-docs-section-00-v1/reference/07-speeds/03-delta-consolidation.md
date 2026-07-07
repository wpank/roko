# Delta — The Consolidation Speed Tier

> Offline reorganization of long-term memory, with no real-time model execution.

**Status**: Shipping
**Crate**: `roko-agent`
**Named after**: Delta waves (0.5–4 Hz) in the EEG — the dominant rhythm during deep
slow-wave sleep, when the hippocampus replays and consolidates memories into cortex.
**Last reviewed**: 2026-04-19

---

## TL;DR

Delta is not a real-time processing speed — it is a scheduled offline pass over the
agent's Substrate. It runs QUERY, SCORE, and PERSIST only (no ACT, no model call). Its
job is to reorganize knowledge: promoting frequently-used Engrams to higher-durability
tiers, pruning stale ones, rebuilding HDC indexes, and updating routing priors. The
Dreams cross-cut runs exclusively at Delta.

---

## What Delta Does and Does Not Do

| Does | Does not |
|---|---|
| Query the full Substrate (up to 24 h lookback) | Call any model or tool |
| Score all Engrams by current relevance/utility | Produce any external output |
| Promote high-utility Engrams to longer half-lives | Accept or process user stimuli |
| Prune Engrams below utility threshold | Run ROUTE, COMPOSE, ACT, or VERIFY |
| Rebuild HDC similarity index | Accept real-time Pulses during execution |
| Update routing priors (CascadeRouter's Wilson CI table) | Block Gamma/Theta processing |
| Integrate Dreams cross-cut (replay and imagination) | — |

Delta runs **concurrently** with Gamma/Theta. A Delta consolidation pass does not
pause the agent's real-time loop — it runs in a background task, writing its results
to the Substrate. Gamma/Theta ticks see the updated Engrams as they arrive.

---

## Triggers

Delta consolidation is triggered by three mechanisms:

### 1. Scheduled interval

Every `delta.interval_hours` (default: 4 h), a Delta pass is scheduled automatically.
This is the "sleep schedule" analog — the agent consolidates regularly regardless of
what happened that day.

### 2. Free energy threshold

When the active inference layer's rolling `free_energy_avg` exceeds
`consolidation_threshold` (default: 0.35), an emergency Delta pass is triggered.
This is the "I'm confused and need to think clearly" signal.

### 3. Manual trigger

An orchestrator or operator may force a Delta pass via the `roko-cli` command:
```
roko agent consolidate --agent-id <id>
```

---

## What a Delta Tick Executes

```
Delta Tick:
  QUERY (full lookback, large candidate cap)
    → 256 Engrams retrieved
  SCORE (by utility + recency + trust)
    → promotion / demotion decisions
  [Dreams cross-cut: replay + imagination]
    → new Engrams generated from replay
  PERSIST (write updated Engrams, prune below threshold)
    → substrate reorganized
  [No ACT, ROUTE, COMPOSE, or VERIFY stages]
```

### Promotion

An Engram is promoted when its utility axis has been consistently high over the last
N ticks. Promotion increases `half_life` — the Engram decays more slowly and will
surface more reliably in future QUERY calls.

### Pruning

An Engram is pruned (its `half_life` reduced to near-zero, triggering rapid decay)
when:
- Its utility axis has been consistently low for K ticks
- It has not been retrieved in the last `pruning_window` (default: 7 days)
- Its trust axis has fallen below the trust floor (e.g., because its source was
  later found to be unreliable)

Pruning is soft: the Engram is marked with `decaying = true` and its half-life is
reduced, but it is not deleted. It will naturally expire over the next few ticks.
Hard deletion is never performed during consolidation — only during explicit storage
reclamation when the substrate approaches its quota.

---

## Dreams Integration

The Dreams cross-cut runs as part of the Delta pass. It has two phases:

1. **NREM replay**: surface Engrams from recent memory (last 24 h) and "replay"
   them — re-score, re-connect to related knowledge, update routing priors.
2. **REM imagination**: generate novel Engrams by combining existing knowledge in new
   ways, using compositional HDC algebra. These are marked `Kind::Imagined` and have
   low initial trust — they are hypotheses, not facts.

See [Dreams cross-cut](../09-cross-cuts/03-dreams.md) for the full implementation.

---

## Parameters

| Parameter | Value | Configurable? |
|---|---|---|
| Interval | 4 h | Yes |
| Substrate lookback | 24 h | Yes |
| Candidate cap | 256 | Yes |
| Promotion threshold | utility_ema > 0.70 for 20 ticks | Yes |
| Pruning threshold | utility_ema < 0.15 for 50 ticks, unseen for 7 d | Yes |
| Concurrency with real-time loop | Yes | No (always concurrent) |

---

## Configuration

```toml
[delta]
interval_hours           = 4
substrate_lookback       = "24h"
candidate_cap            = 256
promotion_threshold      = 0.70
pruning_threshold        = 0.15
pruning_unseen_days      = 7
dreams_enabled           = true
free_energy_trigger      = 0.35    # triggers emergency Delta
```

---

## Observability

| Metric | Description |
|---|---|
| `delta.pass_count` | Total Delta passes completed |
| `delta.engrams_promoted` | Engrams promoted per pass |
| `delta.engrams_pruned` | Engrams marked for pruning per pass |
| `delta.duration_secs` | Wall time per Delta pass |
| `delta.free_energy_before` | Rolling free energy before the pass |
| `delta.free_energy_after` | Rolling free energy after the pass |

A healthy consolidation pass reduces `free_energy_avg` by 20–40%. If it does not
decrease after multiple passes, the agent's world model may have a structural problem
requiring manual intervention.

---

## See also

- [Overview](00-overview.md) — the three-speed system
- [Dreams cross-cut](../09-cross-cuts/03-dreams.md) — offline learning at Delta speed
- [Active Inference](../06-loop/11-active-inference.md) — free energy threshold that triggers Delta
- [Speed Triggers](05-triggers.md) — what activates a Delta pass
- [Resource Budgets](06-resource-budgets.md) — Delta compute budget
