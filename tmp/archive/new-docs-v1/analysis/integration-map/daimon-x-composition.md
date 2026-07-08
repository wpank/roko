---
title: "Daimon × Composition"
section: analysis
subsection: integration-map
id: im-daimon-x-composition
source: 24-cross-section-integration-map.md (§6.2 M2, §3.2)
missing-integration: M2
tier: 1
tags: [daimon, composition, PAD, affect, context-weights, system-prompt]
---

# Daimon × Composition

**Direction**: 09-Daimon → 03-Composition (affect-modulated context assembly)  
**Status**: **Partially Wired** — PAD bias wired into CascadeRouter; SystemPromptBuilder context weights NOT wired (M2 gap)  
**Interface**: `roko-daimon::AffectState` → `roko-compose::SystemPromptBuilder`

## What Flows

Daimon's PAD vector directly modulates which sections of the system prompt receive higher or lower weight. A high-arousal agent should include more safety-relevant context; a low-confidence (low dominance) agent should include more heuristics.

| Signal | From | To | Status |
|---|---|---|---|
| PAD vector | `roko-daimon::AffectState` | `SystemPromptBuilder` section weights | **Missing** (M2 gap) |
| `BehavioralState` | `roko-daimon::AffectState` | Token budget allocation | **Missing** |
| Affect bias for model-tier | `roko-daimon::DaimonPolicy` | `CascadeRouter` | **Wired** |
| Affect bias for context | `roko-daimon` | `SystemPromptBuilder` | **Wired** (partial — limited integration) |

## Wiring Recipe

```rust
// In SystemPromptBuilder::build():
let affect = daimon.current_state();

// High arousal → include more safety context
if affect.pad.arousal > 0.5 {
    builder.set_section_weight("safety_constraints", 1.5);
    builder.set_section_weight("warnings", 1.3);
}

// Low dominance → include more heuristics and workspace map
if affect.pad.dominance < -0.3 {
    builder.set_section_weight("heuristics", 1.4);
    builder.set_section_weight("workspace_map", 1.2);
}

// High pleasure → allow more exploration hints
if affect.pad.pleasure > 0.5 {
    builder.set_section_weight("exploration_hints", 1.3);
}
```

Estimated LOC: ~45 (source file 24, §6.2 M2).

## Invariants of the Interaction

1. Neutral PAD (all axes ≈ 0) produces default section weights — the interaction is additive on top of defaults, never destructive.
2. Weight multipliers are bounded: no section weight below 0.5 or above 2.0 (prevents degenerate prompts).
3. The PAD vector is read once per prompt build, not re-read mid-build.
4. The composition budget (`Budget`) is not modified by affect — only section priority weights change.

## Failure Modes

| Failure | Consequence | Detection |
|---|---|---|
| Daimon returns stale PAD | Composition uses outdated affect | Timestamp check on `AffectState` |
| Section key not found in builder | `set_section_weight` silently no-ops | Log unknown section keys |
| PAD axes out of range [-1, 1] | Weight formula may exceed bounds | Clamp PAD values before applying formula |

## Observed Metrics

Expected once implemented:
- Distribution of section weight multipliers per run
- Correlation between `BehavioralState::Struggling` and safety context injection rate
- Token budget changes attributable to affect modulation

## Open Questions

1. Should the `AffectModel` trait expose raw PAD floats or named predicates (`is_high_arousal()`, `is_low_dominance()`)? Named predicates are more testable.
2. Is there a risk of feedback loop: high arousal → more safety warnings → higher arousal? Need a dampening mechanism.
3. Should affect modulation be logged as a `Kind::Metric` Engram for later analysis?

## Cross-References

- Orchestration direction: [daimon-x-orchestration.md](./daimon-x-orchestration.md) — M1
- Reverse direction: [orchestration-x-daimon.md](./orchestration-x-daimon.md) — M11 closes the loop
- Knowledge context: [neuro-x-composition.md](./neuro-x-composition.md) — M5 (both inject into SystemPromptBuilder; ordering matters)
- Architectural finding: [AA-06: Cross-Cut Isolation](../architectural-analysis/06-finding-crosscut-isolation.md)
- Improvement: [AA-10 I6](../architectural-analysis/10-prioritized-improvements.md) — AffectModel trait
- Readiness audit: [RA-03: Composition](../readiness-audit/subsystem-composition.md), [RA-09: Daimon](../readiness-audit/subsystem-daimon.md)
