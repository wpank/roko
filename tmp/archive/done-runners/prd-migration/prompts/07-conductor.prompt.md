# Prompt: 07-conductor

You are a fresh Claude Opus agent. Zero prior context. Read every file this prompt references before writing.

## Your mission

Generate `/Users/will/dev/nunchi/roko/roko/docs/07-conductor/`. Covers the Conductor as reactive intelligence layer: 10 watchers, 3-state circuit breaker, graduated interventions, diagnosis engine (34 error patterns), stuck detection, health monitors, cybernetic loop, Good Regulator Theorem, Ashby's Law, Yerkes-Dodson dynamics, precision-weighted prediction errors, Cognitive Signals (typed interrupts), adaptive timeouts, graceful shutdown.

## Step 1 — Context pack (MANDATORY)

Read all 7 files in `/Users/will/dev/nunchi/roko/roko/tmp/prd-migration/context-pack/` in order.

## Step 2 — refactoring-prd canonical sources

1. `/Users/will/dev/nunchi/roko/refactoring-prd/02-five-layers.md` §Conductor as Meta-Cognition
2. `/Users/will/dev/nunchi/roko/refactoring-prd/03-cognitive-subsystems.md` §5 cybernetic self-learning, §Self-Model (Good Regulator), §Ashby's Law, §VSM mapping
3. `/Users/will/dev/nunchi/roko/refactoring-prd/09-innovations.md` §XII.2 Cognitive Signals (Pause/Resume/Reprioritize/InjectContext/Escalate/Cooldown/Explore/Shutdown)
4. `/Users/will/dev/nunchi/roko/refactoring-prd/08-translation-guide.md`
5. `/Users/will/dev/nunchi/roko/refactoring-prd/07-implementation-priorities.md` §Tier 1F (Wire conductor watchers)

## Step 3 — SOURCE-INDEX entry `## 07-conductor.md`

Read every file. Key legacy:
- `bardo-backup/prd/25-mori/mori-resilience.md`
- `bardo-backup/prd/13-runtime/21-cybernetic-loops.md`
- `bardo-backup/tmp/mori-refactor/07-orchestration.md` (conductor sections)
- `bardo-backup/tmp/death/21-agent-optimization.md` (extract mechanism, drop mortality framing)
- `bardo-backup/tmp/mori-refactor-plan/10-failure-prevention.md`
- `bardo-backup/tmp/mori-refactor-plan/08-design-principles.md`
- `bardo-backup/tmp/mori-refactor-plan/00-issues-catalog.md` — 21 production failures catalog

## Step 4 — implementation-plans

- `modelrouting/08-learning-loops.md` — circuit breaker (3-state Closed/Open/HalfOpen), anomaly detection
- `modelrouting/16-production-hardening.md` — adaptive timeouts (p95×2), full-jitter backoff, per-provider semaphores, graceful shutdown (3-phase drain), content-addressed dedup cache, hedged requests
- `06-process-management.md`

## Step 5 — active code

- Glob `/Users/will/dev/nunchi/roko/roko/crates/roko-conductor/src/**/*.rs`
- Read: `lib.rs`, `watchers/` (all), `circuit_breaker.rs`, `intervention.rs`, `diagnosis.rs`, `health.rs`

## Step 6 — Output and sub-doc plan

```bash
mkdir -p /Users/will/dev/nunchi/roko/roko/docs/07-conductor
```

Write **15 sub-docs** plus `INDEX.md`:

| # | Filename | Content |
|---|---|---|
| 00 | `00-conductor-as-meta-cognition.md` | The Conductor is NOT just a timeout manager. It's the agent's theory-of-mind about its own pipeline. Models which agents are stronger on which task types. Overview of all capabilities. |
| 01 | `01-ten-watchers.md` | All 10 watchers and what each detects. Watcher trait, lifecycle, event subscriptions. Integration with EventBus from roko-runtime. |
| 02 | `02-3-state-circuit-breaker.md` | Closed → Open → HalfOpen state machine. Error-type-specific cooldowns: RateLimit 5s, Timeout 10s, ServerError 30s, Auth 5min. Health metric thresholds for state transitions. |
| 03 | `03-graduated-interventions.md` | Intervention types: nudge, retry, replan, escalate, abort. When each fires. Gradual escalation strategy. How interventions are coordinated across watchers. |
| 04 | `04-diagnosis-engine.md` | 34 error patterns. Pattern matching against gate failures, error messages, stuck states. Root cause attribution. Fed into replan policy. |
| 05 | `05-stuck-detection.md` | Stuck detection heuristics. Meta-cognition hook ("Am I stuck? Am I thrashing? Should I escalate?" — from 12a I5). Detection of repeated actions, token burn without progress, loops. |
| 06 | `06-health-monitors.md` | Per-agent health metrics. Per-provider health. Aggregate system health. Integration with C-Factor (cross-reference 00-architecture). |
| 07 | `07-cybernetic-loop.md` | Conductor creates closed-loop feedback. Observe → Orient → Decide → Act (OODA). How conductor creates the cybernetic regulatory capacity. |
| 08 | `08-cognitive-signals.md` | Typed interrupts that differ from OS signals: they change agent **behavior** without killing the process. Full enum from 09-innovations.md §XII.2: Pause (suspend reasoning, serialize state), Resume (resume from serialized), Reprioritize(TaskId), InjectContext(Engram), Escalate (switch to stronger model), Cooldown (reduce arousal, slow down), Explore (switch to exploratory mode), Shutdown (graceful termination). Rust enum definition. Use cases. |
| 09 | `09-adaptive-timeouts-backoff.md` | Adaptive timeouts (p95×2 measured per-role/per-model). Full-jitter exponential backoff for retries. Per-provider semaphores. Timeout escalation flow. |
| 10 | `10-graceful-shutdown.md` | 3-phase drain protocol: reject-new → drain-inflight → force-close. Content-addressed dedup cache for inflight tasks. State persistence on shutdown. Signal handling. |
| 11 | `11-good-regulator-theorem.md` | Conant & Ashby 1970 "Every Good Regulator of a System Must Be a Model of That System". Why the agent must model itself to self-regulate effectively. Capability boundaries. Performance trajectory. Theory of mind about other agents (e.g., reviewer agent preferences). Self-model persistence in Neuro as `kind: SelfModel` entries. |
| 12 | `12-ashby-law-requisite-variety.md` | Ashby's Law: regulatory variety must match or exceed environmental variety. Implications: each new failure mode → new Policy implementation. Each new domain → new Gate implementations. Each new model provider → new Backend adapter. The system's regulatory capacity grows organically. |
| 13 | `13-yerkes-dodson-and-precision.md` | Yerkes-Dodson 1908 — moderate pressure maximizes cooperation; extreme pressure causes collapse in 5-12 turns. Dynamic pressure adjustment based on observed cooperation rates. Precision-weighted prediction errors: failure on familiar task = high-precision (learn strongly); failure on novel task = low-precision (learn cautiously). VSM mapping: Conductor = System 3 (internal oversight) + System 3* (audit). |
| 14 | `14-current-status-and-21-failure-catalog.md` | 21 production failure catalog from `bardo-backup/tmp/mori-refactor-plan/00-issues-catalog.md`. Enumerate each with reframed framing. Current status of roko-conductor. Wiring gaps (Tier 1F). Integration points. |

Plus `INDEX.md`.

## Step 7-9 — Rules, INDEX, self-check

Per `context-pack/04-writing-rules.md`. ≥200 lines per sub-doc, ≥3500 total. Citations: Conant & Ashby 1970, Ashby 1956, Beer 1972 (Brain of the Firm — VSM), Wiener 1948, Yerkes-Dodson 1908, OODA loop (Boyd), Song et al. ICLR 2025 (precision-weighted prediction errors).

Cross-reference topics 00-architecture (VSM mapping, Ashby, Good Regulator), 01-orchestration (conductor watches orchestration pipeline), 02-agents (cognitive signals sent to agents), 04-verification (gates trigger conductor reactions), 05-learning (8 feedback loops).

## CRITICAL REMINDERS

- DO NOT SUMMARIZE. DO NOT TRUNCATE. PRESERVE ALL CITATIONS.
- Cognitive Signals are **typed interrupts** for behavior modification, NOT OS signals. They alter agent behavior without killing the process. This is distinct from and more powerful than SIGTERM/SIGKILL.
- The Conductor is a **theory-of-mind about the pipeline**, not just a timeout manager. Make this the main framing.
- Apply naming map: mori → Roko Orchestrator; golem → agent.
- No death framing.
- Use Write tool. Don't ask questions.
