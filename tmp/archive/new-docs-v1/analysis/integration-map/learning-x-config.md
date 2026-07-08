---
title: "Learning × Config"
section: analysis
subsection: integration-map
id: im-learning-x-config
source: 24-cross-section-integration-map.md (§6.1 M10, §5.1)
missing-integration: M10
tier: 2
tags: [learning, config, experiments, static-config, auto-apply, parameter-promotion]
---

# Learning × Config

**Direction**: 05-Learning → 00-Architecture (experiment winners → static config promotion)  
**Status**: **Missing (M10)** — Tier 2, ~90 LOC. `ExperimentStore` exists; no path to promote winners to `roko.toml`.  
**Interface**: `roko-learn::ExperimentStore` → `roko-core::RokoConfig` (config promotion)

## What Flows

The learning system runs A/B experiments on configuration parameters (model selection, gate thresholds, context weights). When an experiment produces a statistically significant winner, it should optionally promote that winner to the static config. Currently experiments inform dashboards but never update `roko.toml`.

| Signal | From | To | Status |
|---|---|---|---|
| Experiment results (winner arm, delta) | `ExperimentStore` | Operator review / promotion | Data only — no code path |
| Promoted experiment winners | `ExperimentStore` | `RokoConfig` | **Missing** (M10) |
| `learning.auto_apply` config flag | Config | Experiment auto-promotion | **Missing** |

## Wiring Recipe

```rust
// In experiment promotion logic (new module in roko-learn):
pub async fn check_and_promote_experiments(
    store: &ExperimentStore,
    config_path: &Path,
    auto_apply: bool,
) -> Result<Vec<PromotionRecord>> {
    let winners = store.get_statistically_significant_winners()?;
    
    let mut promoted = vec![];
    for winner in winners {
        if auto_apply || was_manually_approved(&winner) {
            // Update roko.toml with winning parameter value
            let patch = winner.to_config_patch();
            apply_config_patch(config_path, &patch).await?;
            promoted.push(PromotionRecord::from(&winner));
        }
    }
    Ok(promoted)
}
```

Key: `learning.auto_apply` in `roko.toml` gates whether promotion is automatic or requires manual review. The CLI should have a `roko experiments apply` subcommand for manual review.

Estimated LOC: ~90 (source file 24, §6.1 M10).

See also: feedback loops doc Loop 8 (Experiments→Static is the same gap).

## Invariants of the Interaction

1. Promotion only happens for experiments that have reached statistical significance (`p < 0.05`, minimum sample size met).
2. Config promotion is idempotent — applying the same experiment winner twice produces the same result.
3. Promoted experiments are recorded in `.roko/learn/promotions.jsonl` for auditability.
4. `auto_apply = false` (default) — operators review before promotion; prevents runaway self-modification.
5. Only a bounded set of config parameters are eligible for auto-promotion (allowlist).

## Failure Modes

| Failure | Consequence | Detection |
|---|---|---|
| Config file write fails | Promotion not applied | Log error; mark experiment as "promotion failed" |
| Promoted config causes regression | Runtime quality drops | Config rollback mechanism (keep previous config) |
| Experiment winner based on noisy data | Bad parameter promoted | Statistical significance threshold; minimum sample size |
| `roko.toml` modified externally | Stale config read | Config reload on each session start |

## Observed Metrics

Expected after implementation:
- Experiment promotion rate (% of experiments that produce promotable winners)
- Config parameter change frequency
- Quality delta before/after each promotion (measured by gate pass rate)

## Open Questions

1. Should the `roko experiments list` CLI subcommand show pending promotions with a `--approve` flag?
2. What config parameters should be in the auto-promotion allowlist? Start with `routing.static_table` and `gates.rung_level`?
3. How does this interact with GitOps (if `roko.toml` is in version control)?

## Cross-References

- Skills complement: [learning-x-composition.md](./learning-x-composition.md) — M4 (skills are learned, experiments tune config)
- Readiness audit: [RA-05: Learning](../readiness-audit/subsystem-learning.md), [RA-00: Architecture](../readiness-audit/subsystem-architecture.md)
