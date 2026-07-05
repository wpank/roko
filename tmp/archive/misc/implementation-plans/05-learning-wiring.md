# 05 — Learning & Feedback Wiring

> **Priority**: 🟡 P1 — System works without this but doesn't improve
> **Parity sections**: §16 (Memory), §27 (Self-improvement), I.3 (Learning wiring)
> **Checklist ref**: `MORI-PARITY-CHECKLIST.md` §16, §27, I.3
> **Status**: ✅ Completed (2026-04-08)

## Problem statement

Roko has episode logging, skill library, costs DB, and provider health tracking
all implemented in `roko-learn` (`/Users/will/dev/nunchi/roko/roko/crates/roko-learn/`),
but the orchestrator/CLI never calls them. Agents run, produce output, and the
output is discarded with no learning.

## What exists (built and wired)

| Component | Path | Wired? |
|-----------|------|--------|
| Episode logger | `roko-learn/src/episode_logger.rs` | ✅ |
| Skill library | `roko-learn/src/skill_library.rs` | ✅ |
| Costs DB | `roko-learn/src/costs_db.rs` | ✅ |
| Provider health tracker | `roko-learn/src/provider_health.rs` | ✅ |
| Playbook scoring | `roko-learn/src/playbook.rs` | ✅ |
| Regression detector | `roko-learn/src/regression.rs` | ✅ |
| Efficiency events | `roko-learn/src/efficiency.rs` | ✅ |
| Cascade router | `roko-learn/src/cascade_router.rs` | ✅ |
| Prompt experiments | `roko-learn/src/prompt_experiment.rs` | ✅ |
| Adaptive thresholds | `roko-gate/src/adaptive_threshold.rs` | ✅ |
| Runtime feedback | `roko-learn/src/runtime_feedback.rs` | ✅ |

## Checklist

### Phase A: Episode persistence

- [x] **5.1** After agent run completes, persist Episode to `.roko/learn/episodes.jsonl`
- [x] **5.2** Episode includes: prompt, response, gate results, cost, duration, model, role
- [x] **5.3** `orchestrate.rs` calls `LearningRuntime::record_completed_run` at end of run

### Phase B: Cost tracking

- [x] **5.4** Parse token usage from agent response
- [x] **5.5** Write to costs DB after each agent call
- [x] **5.6** Wire cost data into prompt budget decisions

### Phase C: Skill extraction

- [x] **5.7** After successful gate pass, extract skill from episode
- [x] **5.8** Store skills in skill library
- [x] **5.9** Inject relevant skills into future system prompts (via `build_learned_context`)
- [x] **5.9b** Emit `AgentEfficiencyEvent` per agent turn (`.roko/learn/efficiency.jsonl`)

### Phase D: Feedback loops

- [x] **5.10** Provider health: track latency, error rate per model/provider
- [x] **5.11** Regression detector: compare recent episodes against baseline
- [x] **5.12** Playbook scoring: update playbook weights based on outcomes

### Phase E: Self-improvement (§27)

- [x] **5.13** Prompt optimization loop (A/B test prompt variants via `ExperimentStore`)
- [x] **5.14** Model routing optimization (CascadeRouter with disk persistence + configurable models)
- [x] **5.15** Gate threshold tuning (AdaptiveThresholds with EMA per rung)

> Maps to checklist: I.3.1 through I.3.8, §16.1-16.8, §27.1-27.10
