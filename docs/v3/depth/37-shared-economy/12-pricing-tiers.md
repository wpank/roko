# Depth: Pricing Tiers

> Reputation-driven commercial pricing: five tiers from Free to Enterprise,
> discipline override, and the aggregate score computation.

**Parent chapter:** [37-SHARED-ECONOMY](../../37-SHARED-ECONOMY.md) Section 7
**Source:** `crates/roko-chain/src/reputation_registry.rs`

---

## Tier Resolution

Pricing tiers determine what an agent can charge for paid feeds and services.
The resolution computes an aggregate score from all seven domain reputations
and maps it to a tier:

```rust
pub fn resolve_pricing_tier(
    registry: &ReputationRegistry,
    passport_id: AgentId,
    now: u64,
) -> PricingTierResult {
    let aggregate_score = REPUTATION_DOMAINS.iter()
        .map(|domain| registry.get_score(passport_id, domain, now))
        .sum::<f64>()
        / REPUTATION_DOMAINS.len() as f64;
    let discipline = registry.discipline_state(passport_id, now);
    pricing_tier_for(aggregate_score, discipline)
}
```

### Aggregate Score

The aggregate is the arithmetic mean of all seven effective (decay-adjusted)
domain scores. An agent active in only one domain still has six neutral (0.5)
scores, so specialization alone does not reach Enterprise tier:

- One domain at 1.0, six at 0.5: aggregate = (1.0 + 6*0.5) / 7 = 0.571 (Standard)
- Three domains at 0.9, four at 0.5: aggregate = (3*0.9 + 4*0.5) / 7 = 0.671 (Professional)
- All seven at 0.8: aggregate = 0.800 (Enterprise)

This incentivizes breadth: Enterprise-tier agents must demonstrate quality
across multiple domains.

---

## Tier Boundaries

```rust
fn pricing_tier_for(aggregate_score: f64, discipline: DisciplineState) -> PricingTierResult {
    let (tier_name, price_multiplier) = if discipline != DisciplineState::GoodStanding {
        ("Free", 0.0)
    } else if aggregate_score < 0.4 {
        ("Starter", 0.5)
    } else if aggregate_score < 0.6 {
        ("Standard", 1.0)
    } else if aggregate_score < 0.8 {
        ("Professional", 1.5)
    } else {
        ("Enterprise", 2.0)
    };

    PricingTierResult {
        tier_name: tier_name.to_string(),
        price_multiplier,
        aggregate_score,
        discipline: format!("{discipline:?}"),
    }
}
```

| Aggregate | Discipline | Tier | Multiplier | Meaning |
|---|---|---|---|---|
| Any | Not GoodStanding | Free | 0.0x | Cannot sell paid access |
| < 0.4 | GoodStanding | Starter | 0.5x | Half of base price |
| 0.4 - 0.6 | GoodStanding | Standard | 1.0x | Base price |
| 0.6 - 0.8 | GoodStanding | Professional | 1.5x | 50% premium |
| >= 0.8 | GoodStanding | Enterprise | 2.0x | 2x premium |

---

## Discipline Override

Discipline state takes absolute precedence over aggregate score. An agent
with aggregate score 0.95 but in Probation receives the Free tier:

```rust
if discipline != DisciplineState::GoodStanding {
    ("Free", 0.0)
}
```

This means:
- A single domain dropping below 0.4 can demote an otherwise excellent agent
  to Free tier
- Recovery from probation restores the score-based tier immediately
- The override prevents agents with quality issues from charging premium prices

---

## Integration with Payments

The `price_multiplier` is applied to a feed's base price when computing what
an agent charges subscribers:

```
agent_price = feed_base_price * price_multiplier
```

A Professional agent (1.5x) charges 50% more than a Standard agent (1.0x) for
the same feed. An Enterprise agent (2.0x) charges double. A Free agent (0.0x)
cannot charge at all.

This creates a direct economic incentive to build and maintain reputation:
higher reputation enables higher prices.
