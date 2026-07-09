# Learning & Feedback Subsystem Audit

Episodes, CascadeRouter, efficiency events, experiments, playbooks, conductor, budget — 10 learning components that form a closed loop. All fully built and wired. The catch: they're only wired from orchestrate.rs (dead code).

## The Problem

The learning subsystem is the most sophisticated part of roko. CascadeRouter (LinUCB bandit), prompt experiments (A/B testing), playbook extraction, conductor retry policy, budget guardrails — all production-ready, all persisting, all reading back learned state. But the **entire closed loop only runs from `orchestrate.rs`**, which is dead code. Live paths (`roko run`, `roko chat`, ACP) record nothing durable. The system cannot learn from 99% of its actual runs.

---

## 1. Component Status

| Component | Built? | Wired? | Live Callers? | Persistence |
|---|---|---|---|---|
| Episode logger | Yes | Yes | orchestrate.rs only (dead) | `.roko/episodes.jsonl` |
| CascadeRouter | Yes | Yes | orchestrate.rs only (dead) | `.roko/learn/cascade-router.json` |
| Efficiency events | Yes | Yes | orchestrate.rs only (dead) | `.roko/efficiency.jsonl` |
| Prompt experiments | Yes | Yes | orchestrate.rs only (dead) | `.roko/learn/model-experiments.json` |
| Playbook store | Yes | Yes | orchestrate.rs only (dead) | `.roko/learn/playbooks/*.json` |
| Conductor bandit | Yes | Yes | orchestrate.rs only (dead) | `.roko/learn/conductor.json` |
| Budget guardrails | Yes | Yes | orchestrate.rs only (dead) | Config + runtime |
| Adaptive thresholds | Yes | Yes | ACP (basic), orchestrate.rs (full) | `.roko/learn/gate-thresholds.json` |
| Cost tracking | Yes | Yes | orchestrate.rs only (dead) | `.roko/learn/costs.jsonl` |
| Knowledge routing | Yes | Partial | orchestrate.rs only (dead) | Neuro store queries |

**Bottom line:** 10/10 components fully built and wired. 1/10 has a live caller (adaptive thresholds via ACP, basic mode only). The rest only run from dead code.

---

## 2. Episode System

**What's recorded per episode:**
- Agent identity (role, agent_id, template)
- Task context (plan_id, task_id, domain)
- Token usage (input, output, cached, cost_usd)
- Duration (ttft_ms, wall_ms)
- Gate verdicts (ordered pass/fail per gate)
- Prompt composition snapshot (sections, token counts, truncations)
- Model, provider, backend
- HDC fingerprint
- Emotional tags (daimon state)
- Success flag + failure reason

**Written by:** `learning.append_episode(&ep)` — called from 12+ locations in orchestrate.rs

**Read by:** `learning.record_completed_run()` which triggers:
- CascadeRouter observation updates
- Cost record append
- Skill extraction (SkillLibrary)
- Neuro distillation hook
- Pattern discovery
- Playbook rule confidence updates
- Anomaly detection
- Regression detection

**Live callers:** Zero. `roko run`, `roko chat`, `roko "prompt"`, ACP — none write episodes.

---

## 3. CascadeRouter (LinUCB Bandit)

**Architecture:**
- Arms: all configured model slugs
- Context features (11 dimensions): task_category, role, frequency, model_tier, cfactor, cost_pressure, latency_pressure, conductor_load, provider_health, cost_spike, behavioral_state_shift
- Reward: multi-objective via `compute_routing_reward_v2()` (success + latency + cost + c-factor)

**Called from:** `cascade_router.select_for_frequency_among()` at orchestrate.rs:14264

**Updated from:** `observe_multi_objective()` at orchestrate.rs:10356 after each gate verdict

**Live callers:** Zero. `dispatch_direct.rs` uses hardcoded model defaults.

**Persistence:** `.roko/learn/cascade-router.json` — saved after each observation.

---

## 4. Efficiency Events

**Per-turn record with 20+ fields:**
- Agent identity + model + backend
- Token accounting (input, output, cached, reasoning)
- Cost accounting (per-model pricing)
- Prompt composition metadata (section names, tokens, priority, truncation)
- Tool calls (name, duration, result_tokens, success, redundancy)
- Timing (duration, processing stages)
- Quality (efficiency score, quality grade, tool effectiveness)

**Written to:** `.roko/efficiency.jsonl`

**Read by:** Conductor (cost pressure signals), dashboard, learning runtime

**Live callers:** Zero for writes. Dashboard reads the file if it exists from a previous orchestrate.rs run.

---

## 5. Prompt Experiments (A/B Testing)

**How it works:**
- `ExperimentStore` in `.roko/learn/model-experiments.json`
- Multiple `PromptVariant` per experiment with UCB1/Thompson arm selection
- Per-variant stats: trials, successes → Wilson confidence interval
- Bandit-driven exploration → convergence → concluded winners applied as static overrides

**Called from:** `experiment_store.assign_model_with_experiment()` at orchestrate.rs:14242

**Live callers:** Zero.

---

## 6. Playbook Store

**What playbooks are:** Named sequences of proven action steps extracted from successful tasks.

**Lifecycle:**
1. Task succeeds → `build_task_playbook()` extracts playbook from task definition
2. Saved to `.roko/learn/playbooks/{id}.json`
3. At dispatch time: `playbook.query(&query)` returns matching playbooks
4. Playbooks injected into system prompt Layer 6 (relevant techniques)
5. After gate: `playbook.record(task_id, success)` updates confidence

**Live callers:** Zero. Playbooks are never created or queried from live paths.

---

## 7. Conductor Bandit (Retry Policy)

**7 actions:** Continue, InjectHint(ErrorDigest), InjectHint(SkillSuggestion), InjectHint(SimplifyApproach), SwitchModel, Restart, Abort

**19-dimension context:** iteration, consecutive_failures, error_pattern, elapsed_ms, cost_so_far, model_tier, task_complexity

**Called from:** orchestrate.rs retry loop (lines 8974, 9117/9153)

**Live callers:** Zero. Live paths don't have retry loops with learned intervention.

---

## 8. Budget Guardrails

**Limits:** `budget.max_task_usd`, `budget.max_session_usd`, `budget.max_plan_usd`

**Enforcement:**
1. Pre-dispatch: `ensure_task_budget_available()` — fail if exhausted
2. Pre-routing: `routing_budget_pressure()` — compute pressure factor
3. Routing bias: conductor evaluates signals, applies pressure
4. Guardrail check: `BudgetGuardrail::record_cost()` → Allow/Warn/RouteToCheaper/Block

**Live callers:** Zero. `dispatch_direct.rs` has no budget enforcement.

---

## 9. The Closed Loop (Only In Dead Code)

```
dispatch_agent_with()
├─ Consult CascadeRouter → select model
├─ Check budget → routing pressure
├─ Load experiment → variant override
├─ Query playbooks → prompt injection
├─ Build 9-layer prompt → PromptComposer
├─ Dispatch agent
├─ Record episode → EpisodeLogger
├─ Record efficiency → efficiency.jsonl
├─ Observe routing → CascadeRouter update
├─ Record outcome → playbook update
├─ Record experiment → variant stats
├─ Update conductor → retry policy
├─ Update thresholds → gate learning
├─ Trigger distillation → knowledge store
└─ Check replan → gate failure response
```

Every step feeds the next. The system improves with every run. **But this entire loop is dead.**

---

## 10. What Live Paths Record

| Signal | `roko run` | `roko chat` | `roko "prompt"` | ACP |
|---|---|---|---|---|
| Episode | No | No | No | No |
| Efficiency event | No | No | No | No |
| Cost record | No | No | No | No |
| Routing observation | No | No | No | No |
| Playbook update | No | No | No | No |
| Budget check | No | No | No | No |
| CostMeter (session) | No | No | In-memory | No |
| Gate thresholds | No | No | No | Basic EMA |

---

## 11. Anti-Patterns

| Anti-Pattern | Where |
|---|---|
| **#6 Feedback as afterthought** | All live paths record zero learning signals |
| **#3 Build another runtime** | Learning wired to dead runtime, not migrated to live ones |
| **#10 God file** | All 10 learning components called from orchestrate.rs (21K lines) |

---

## 12. What FeedbackService Should Do

Every model call, regardless of entry point, should automatically:
1. Record episode (who, what model, prompt sections, tokens, cost, duration)
2. Observe routing outcome (model chosen, success/failure → CascadeRouter)
3. Record cost (model, tokens, cost_usd)
4. Update playbook confidence (if task matched a playbook)
5. Update gate thresholds (if gates ran)
6. Check budget (per-task and per-session limits)

This is Phase 0.3 of the unified plan. The learning code is ready — it just needs callers from live paths.
