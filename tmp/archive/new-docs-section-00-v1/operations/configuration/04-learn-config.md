# Learn Configuration

> The `[learn]` table controls Roko's learning and adaptation layer: the CascadeRouter
> that routes tasks to appropriate model tiers, the A/B testing bandits, the episode
> store, and the playbook pattern extractor.

**Status**: Shipping (cascade_router, experiments, episode_store, playbook_path) / Built (distillation)
**Crate**: `roko-learn`
**Depends on**: [01-roko-toml-schema.md](01-roko-toml-schema.md)
**Used by**: [operations/performance/09-scaling-patterns.md](../performance/09-scaling-patterns.md)
**Last reviewed**: 2026-04-19

---

## TL;DR

The learning runtime runs in the background and improves Roko's behaviour over time.
The most impactful setting is `cascade_router = true`, which can reduce average inference
cost by routing simple tasks to cheaper models automatically.

```toml
[learn]
cascade_router = true
experiments    = true
```

---

## CascadeRouter (T0 → T1 → T2)

The `cascade_router` key enables the three-tier model routing system:

| Tier | Description | Latency | Cost |
|------|-------------|---------|------|
| T0 | Deterministic rule matching (no LLM) | < 1 ms | $0 |
| T1 | Fast, cheap model (e.g. `claude-haiku-4-5`) | ~0.5 s | ~$0.001 |
| T2 | Full model from `agent.model` | ~2–5 s | ~$0.01–0.05 |

When `cascade_router = true`:

1. A task arrives at the router.
2. T0 rules are checked first (pattern matching against task type, keywords, history).
   If T0 fires (e.g. a trivial one-line config change), the task is handled without
   calling any LLM.
3. If T0 is uncertain, T1 (a cheap fast model) is invoked to classify and attempt the
   task.
4. If T1's output fails the gate pipeline, the task escalates to T2 (the full model).

Tier escalation only happens on gate failure — not on timeout or other errors. If T1
succeeds (passes all gates), the task is done. This routing learns over time: the
Thompson Sampling bandit in `roko-learn` tracks which tier succeeds for each task
category and adjusts routing probabilities accordingly.

**Disable the router to always use the full model:**

```toml
[learn]
cascade_router = false
```

Useful when you need maximum quality for all tasks and cost is not a concern.

---

## A/B Testing Experiments

When `experiments = true`, the learning runtime maintains prompt variant bandits for each
task category. On each task execution:

1. Thompson Sampling selects either the current best prompt variant (exploitation) or a
   random variant (exploration).
2. The variant is injected into the agent's system prompt for this task.
3. The outcome (gate pass/fail, gate pass rate, token count) is recorded.
4. The bandit updates its posterior over variants.

Over time, the best-performing variant for each task category is selected more and more
often. The worst-performing variants are effectively retired.

**Disable experiments for reproducible, deterministic runs:**

```toml
[learn]
experiments = false
```

Always disable experiments in CI or audited environments where you need consistent
behaviour across runs.

---

## Episode Store

Every task execution writes an episode record to the `episode_store` directory. An
episode contains:

- Task type and category.
- Model tier used (T0/T1/T2) and model slug.
- Token counts (input and output).
- Cost in USD.
- Gate pass/fail verdict.
- Number of retries.
- HDC fingerprint (10,240-bit hypervector) of the task's structural signature.

Episodes are the raw material for the pattern extractor. When 5+ similar episodes share
a common outcome, the extractor promotes a pattern to the playbook.

**Moving the episode store to a shared path (multi-user server):**

```toml
[learn]
episode_store = "/var/roko/episodes"
```

All users whose Roko instances point at the same episode store contribute to shared
learning. The store is append-only (one JSONL record per episode); concurrent writes
are safe.

---

## Playbook

The playbook is a TOML file of promoted patterns — rules extracted from episodes that
correctly predict outcomes across multiple builds. Playbook rules are injected directly
into agent context at task start, giving agents immediate access to hard-won heuristics.

**Example playbook entry (generated automatically by the pattern extractor):**

```toml
[[rules]]
category    = "rust-type-definition"
description = "When adding a new field to a shared struct, always check all match arms in the codebase before compiling"
confidence  = 0.94
evidence    = 12
since       = "2026-03-10"
```

Commit the playbook to version control to share learned patterns across the team.

```toml
[learn]
playbook_path = "roko-playbook.toml"
```

---

## Pattern Extraction Threshold

```toml
[learn]
min_episodes_for_pattern = 5
```

Lower this value to extract patterns faster at the cost of less evidence per pattern.
Raise it for a slower but more reliable playbook.

---

## Distillation (Built, not default)

Knowledge distillation compresses the episode store into a condensed playbook
representation using a cheap LLM pass. It runs on a background schedule and reduces
episode store growth over long-lived deployments.

```toml
[learn]
distillation = true
```

**Status: Built** — The code exists and is tested, but is not enabled by default pending
more production validation. Enable it in non-critical deployments to help test.

---

## Two Full Examples

**Laptop developer (learning on, experiments on):**

```toml
[learn]
cascade_router           = true
experiments              = true
episode_store            = ".roko/episodes"
playbook_path            = ".roko/playbook.toml"
min_episodes_for_pattern = 5
distillation             = false
```

**Team server (shared learning store, no experiments for reproducibility):**

```toml
[learn]
cascade_router           = true
experiments              = false
episode_store            = "/var/roko/shared/episodes"
playbook_path            = "/var/roko/shared/playbook.toml"
min_episodes_for_pattern = 8
distillation             = false
```

---

## See Also

- [reference/06-loop/](../../reference/06-loop/README.md) — the cognitive loop that learning hooks into
- [operations/performance/08-regression-detection.md](../performance/08-regression-detection.md) — how roko-learn detects performance regressions
- [01-roko-toml-schema.md](01-roko-toml-schema.md) — full key reference

## Open Questions

- `learn.router_model` key (which model to use for T1 tier) is not yet in the schema — defaults to the smallest/cheapest model for the selected backend.
- Per-category routing overrides (force T2 for "architectural" tasks) are not yet configurable.
