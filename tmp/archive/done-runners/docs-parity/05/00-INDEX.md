# 05-Learning Parity Refresh

Audit-corrected parity view of `docs/05-learning/` against the live learning stack.

Generated: 2026-04-18

---

## Runtime Picture

- `roko-learn` is already a large shipped subsystem: **42 modules, 35,847 LOC**.
- The live stack already includes `episode_logger`, `pattern_discovery`, `active_inference`, `prediction`, `cascade_router`, `prompt_experiment`, `runtime_feedback`, `efficiency`, and `regression`.
- `roko-neuro` already has **tier progression**; knowledge tiers are not a doc-only concept.
- The main parity problem is no longer "missing learning architecture." It is **docs and production callsites understating or underusing learning code that already exists**.

## What This Refresh Changes

- narrows the batch from "build the grand learning vision" to "describe the shipped runtime honestly",
- separates **shipping**, **ship soon**, and **deferred** work,
- keeps the highest-value follow-ups explicit:
  - add an HDC fingerprint field to `Engram`,
  - harden learned-context matching,
  - make regression output slice-aware,
  - choose one canonical predictive-calibration path,
  - treat typed heuristic calibration as a near-term follow-up,
- demotes demurrage, worldviews, replication-ledger ideas, constitutional constraints, and scaling theses to explicit future work.

## Current Parity Posture

### Shipping

- episode logging, compaction, and pattern mining
- playbooks, playbook rules, skill extraction, prompt injection
- UCB1, Track-and-Stop, LinUCB, cascade routing, active-inference tier selection
- efficiency events, task metrics, cost normalization, provider health, latency tracking
- `LearningRuntime` as the integration hub
- routing-log-backed predictive calibration consumers

### Ship Soon

- HDC fingerprint on `Engram` as the bridge between `roko-core`, `roko-neuro`, and `roko-learn`
- typed heuristic calibration struct layered onto the existing heuristic and tier-progression flow
- making learned-context and regression outputs use richer signals already present in code

### Deferred

- demurrage as the canonical memory model
- worldview clustering, replication-ledger, and constitutional constraints
- FEP/VSM/Friston framing as an implementation doctrine
- c-factor as the canonical optimization target
- exponential scaling / autocatalytic claims as engineering guidance

## File Guide

| File | Focus | Refresh Outcome |
|------|-------|-----------------|
| [A-episodes-patterns.md](A-episodes-patterns.md) | episodes + pattern discovery | treat `EpisodeLogger`, `PatternMiner`, and k-medoids as live; mark tiered storage and DBSCAN as planned |
| [B-knowledge-tiers.md](B-knowledge-tiers.md) | playbooks, skills, tier progression | note that tier progression already exists in `roko-neuro`; narrow the real gap to learned-context wiring |
| [C-routing-bandits.md](C-routing-bandits.md) | routers + bandits | center the shipped cascade/UCB1/active-inference path; defer research routers |
| [D-metrics-cost-health.md](D-metrics-cost-health.md) | efficiency, cost, health | emphasize that efficiency events and cost normalization already ship; narrow the gaps to regression output quality |
| [E-feedback-calibration.md](E-feedback-calibration.md) | runtime feedback + predictive calibration | note that `prediction.rs`, `drift.rs`, and `regression.rs` are real; pick one canonical calibration story |
| [F-frameworks-vision.md](F-frameworks-vision.md) | academic framing + future vision | keep framework mappings, demote worldview / replication-ledger / constitutional layers |
| [BATCHES.md](BATCHES.md) | execution contract | shrink future batches to single-agent 45-90 minute slices |
| [SOURCE-INDEX.md](SOURCE-INDEX.md) | code anchors | refresh stale line references and include current learning/tier-progression anchors |

## Recommended Reading Order

1. [SOURCE-INDEX.md](SOURCE-INDEX.md)
2. [A-episodes-patterns.md](A-episodes-patterns.md)
3. [B-knowledge-tiers.md](B-knowledge-tiers.md)
4. [D-metrics-cost-health.md](D-metrics-cost-health.md)
5. [E-feedback-calibration.md](E-feedback-calibration.md)
6. [F-frameworks-vision.md](F-frameworks-vision.md)

## Practical Reading Rule

If a learning doc claims a system is "wired", this refresh now expects one of two things:

- a real runtime caller in `crates/`, or
- an explicit `planned` / `target-state` label.

Anything else should be treated as stale.
