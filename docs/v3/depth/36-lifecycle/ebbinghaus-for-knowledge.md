# Ebbinghaus for Knowledge, Not Agents

> **v3 depth file** -- `/docs/v3/depth/36-lifecycle/ebbinghaus-for-knowledge.md`
> Canonical source: v1 `docs/v1/17-lifecycle/10-ebbinghaus-for-knowledge-not-agents.md`
> Status: **Current** (Ebbinghaus decay drives per-entry confidence erosion
> with four-tier modulation in `roko-neuro`; balance-based demurrage and tier
> progression live)

---

## 1. The Ebbinghaus Forgetting Curve

Hermann Ebbinghaus (1885) demonstrated that memory retention follows a
negative exponential decay: recently learned information fades rapidly at
first, then more slowly over time:

```
retention = e^(-t / (strength * scale))
```

where `t` is time since last access, `strength` is how well the memory was
encoded, and `scale` is a time constant.

**In roko, Ebbinghaus applies to knowledge only -- never to agent lifespan.**
Signals in the knowledge store decay according to the Ebbinghaus curve, but
the agent itself does not die from knowledge staleness. Staleness triggers
tier demotion, affect state transitions, and eventually archival -- but the
agent continues running. The user decides when to delete.

---

## 2. The Decay Enum

Four decay variants are available on Signals:

```rust
pub enum Decay {
    /// No decay. Confidence remains constant.
    /// Use: architectural facts, mathematical constants.
    None,

    /// Exponential half-life decay.
    /// confidence(t) = initial * 0.5^(t / half_life_ms)
    /// Use: time-sensitive observations with known shelf life.
    HalfLife { half_life_ms: u64 },

    /// Time-to-live. Binary: full confidence until TTL, then 0.
    /// Use: ephemeral data (price quotes, gas estimates).
    Ttl { expires_at: u64 },

    /// Ebbinghaus forgetting curve.
    /// retention = e^(-t / (strength * scale_ms))
    /// Use: most knowledge types (insights, heuristics, warnings).
    Ebbinghaus { strength: f64, scale_ms: u64 },
}
```

`Decay::Ebbinghaus` is the default for most knowledge types. The `strength`
parameter encodes how well the Signal was encoded. The `scale_ms` parameter
is the time constant.

---

## 3. Tier-Modulated Decay

Signal decay rate is modulated by knowledge tier. Higher tiers decay more
slowly because they represent more thoroughly validated knowledge:

| Tier | Multiplier | Effective Decay | Meaning |
|------|-----------|----------------|---------|
| Transient | 0.1x | Very fast | Recently created, unvalidated |
| Working | 0.5x | Moderate | Used but not consolidated |
| Consolidated | 1.0x | Standard | Validated through experience |
| Persistent | 5.0x | Very slow | Repeatedly validated, high confidence |

The effective decay formula:

```
effective_half_life = tier_multiplier * type_base_half_life
```

### Knowledge Type Base Half-Lives

| Knowledge Type | Base Half-Life | Rationale |
|---------------|---------------|-----------|
| Insight | 168 hours (1 week) | Observations and interpretations |
| Heuristic | 336 hours (2 weeks) | Rules of thumb, longer if validated |
| Warning | 72 hours (3 days) | Safety-critical, short to prevent stale warnings |
| CausalLink | 504 hours (3 weeks) | Causal relationships, structural knowledge |
| StrategyFragment | 168 hours (1 week) | Tactical, regime-sensitive |
| AntiKnowledge | 720 hours (30 days) | "What doesn't work" -- stable knowledge |

### Example Decay Rates

A Warning Signal at Transient tier:
- Base half-life: 72 hours. Tier multiplier: 0.1x.
- Effective half-life: 7.2 hours.
- Loses half its confidence in ~7 hours unless retrieved.

A CausalLink Signal at Persistent tier:
- Base half-life: 504 hours. Tier multiplier: 5.0x.
- Effective half-life: 2,520 hours (~105 days).
- Retains confidence for months.

---

## 4. The Testing Effect: Retrieval Counteracts Decay

Roediger & Karpicke (2006) demonstrated that retrieving information
strengthens the memory trace more effectively than re-studying. In roko,
every time a Signal is retrieved and used in a cognitive loop iteration, its
`strength` parameter increases:

- Base strength increase from retrieval: +0.05.
- Gate-pass bonus (Signal used in a verified turn): +0.03.
- Diversity bonus (retrieved under diverse conditions): +0.02 * diversity.
- Maximum strength: 10.0.

Each retrieval also resets `ticks_since_access`, restarting the decay clock.

This creates natural selection on knowledge: Signals that are frequently
retrieved and prove useful accumulate strength and resist decay. Unused
Signals decay. The knowledge store self-prunes without explicit deletion.

---

## 5. Why Ebbinghaus for Agent Lifespan Was Wrong

The legacy system used epistemic fitness (prediction accuracy) as an agent
death clock. The research grounding was sound -- 91% of ML models degrade
temporally (Vela et al. 2022), knowledge has measurable half-lives (Arbesman
2012), concept drift formalizes decay (Zliobaitе et al. 2014). The error was
applying these findings to agent lifespan instead of knowledge management.

### The Category Error

When a biological organism's cells accumulate damage, the organism dies
because repair mechanisms have finite fidelity. When an agent's knowledge
becomes stale, the knowledge can be refreshed, replaced, or restored --
because knowledge is digital, not physical. The "damage" is fully reversible.
You do not need to kill the agent to fix stale knowledge:

1. Let Ebbinghaus decay naturally prune stale Signals.
2. Run Dream consolidation to reorganize and refresh knowledge.
3. Restore fresh knowledge from a backup or mesh.
4. Delete stale Signals and let the agent re-learn.

### What Knowledge Decay Achieves

| Benefit attributed to agent death | How knowledge decay achieves it |
|----------------------------------|-------------------------------|
| Stale knowledge purged | Ebbinghaus reduces confidence of unused Signals |
| Active exploration incentivized | Only fresh evidence maintains confidence |
| Knowledge sharing before loss | Signals approaching archive are prime for sharing |
| Lean, current knowledge base | Natural turnover without explicit deletion |
| Domain-specific decay rates | Per-type half-lives match domain volatility |

### What Knowledge Decay Avoids

| Problem with agent death | How knowledge decay avoids it |
|-------------------------|------------------------------|
| Arbitrary termination | No stochastic death clock |
| Terminal state behavioral distortion | No "dying agent" behavior |
| Succession overhead | No death protocol, no testament |
| User frustration | User controls lifecycle |
| Category error | Decay applies to knowledge, not processes |

---

## 6. Domain-Specific Decay Rates

Following Arbesman's research (2012), the knowledge store supports
domain-specific decay multipliers:

```toml
[neuro.domain_decay]
gas_patterns = 2.0       # Decays 2x faster
price_direction = 1.5    # Decays 1.5x faster
volatility_regime = 1.0  # Standard rate
yield_trends = 0.8       # Decays 0.8x rate
protocol_behavior = 0.5  # Decays 0.5x rate (most stable)
```

A chain-domain agent monitoring gas markets needs fast gas decay (patterns
change hourly). A research agent tracking scientific literature needs slow
decay (findings are relatively stable).

---

## 7. Tier Promotion and Demotion

Signals move between tiers based on validation:

**Promotion** (requires active validation):
```
Transient -> Working:      Retrieved and used in 3+ gate-passed turns
Working -> Consolidated:   Validated through 10+ independent experiences
Consolidated -> Persistent: Confirmed across 3+ distinct contexts
```

**Demotion** (automatic via Ebbinghaus decay):
```
Persistent -> Consolidated: Confidence drops below 0.6
Consolidated -> Working:    Confidence drops below 0.4
Working -> Transient:       Confidence drops below 0.2
Transient -> Archived:      Confidence drops below 0.05
```

Demotion happens automatically. Promotion requires active use and gate
verification.

---

## 8. Ebbinghaus Decay Implementation

```rust
pub fn ebbinghaus_retention(
    time_since_access_ms: u64,
    strength: f64,
    scale_ms: u64,
) -> f64 {
    let t = time_since_access_ms as f64;
    let denominator = strength * scale_ms as f64;
    if denominator <= 0.0 { return 0.0; }
    (-t / denominator).exp()
}

pub fn effective_confidence(signal: &Signal) -> f64 {
    match signal.decay.model {
        DecayModel::None => signal.score.confidence,
        DecayModel::HalfLife { half_life_ms } => {
            let t = ticks_to_ms(signal.decay.ticks_since_access);
            signal.score.confidence * 0.5_f64.powf(t / half_life_ms as f64)
        }
        DecayModel::Ttl { expires_at } => {
            if now_ms() > expires_at { 0.0 } else { signal.score.confidence }
        }
        DecayModel::Ebbinghaus { strength, scale_ms } => {
            let t = ticks_to_ms(signal.decay.ticks_since_access);
            let retention = ebbinghaus_retention(t as u64, strength, scale_ms);
            signal.score.confidence * retention * signal.decay.tier_multiplier
        }
    }
}
```

---

## 9. Implementation Sources

| Surface | File | What |
|---------|------|------|
| Decay enum | `crates/roko-core/src/engram.rs` | Signal decay variants |
| Ebbinghaus computation | `crates/roko-neuro/src/` | Retention calculation |
| Tier management | `crates/roko-neuro/src/` | Promotion, demotion |
| Testing effect | `crates/roko-learn/src/` | Retrieval strengthening |

---

## Cross-References

- [knowledge-demurrage.md](knowledge-demurrage.md) -- Token-level knowledge decay
- [academic-foundations.md](academic-foundations.md) -- Ebbinghaus (1885), Roediger (2006)
- [selective-restore.md](selective-restore.md) -- Decay on restored knowledge
