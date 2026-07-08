# The Universal Loop, Retold

> **TL;DR**: The nine-step loop in `docs/00-architecture/09-universal-cognitive-loop.md`
> collapses to seven when Pulse and Bus are first-class, and it stops
> lying about the three ways the runtime actually perceives.

> **For first-time readers**: Roko's existing architecture doc
> (`docs/00-architecture/09-universal-cognitive-loop.md`) describes a nine-step
> loop the runtime walks every tick: PERCEIVE, EVALUATE, ATTEND, INTEGRATE,
> ACT, VERIFY, PERSIST, ADAPT, META-COGNIZE. This doc argues it should be seven
> steps (two collapse, one turns out to be a cross-cut rather than a step, and
> one splits into co-equal persist/broadcast). Read 01–04 first to get the
> two-medium vocabulary; this is where it shows up in the runtime's heartbeat.

## 1. What's wrong with the current nine steps

The current loop (from doc 09 §1):

```
1. PERCEIVE      → Substrate.query()       What is happening?
2. EVALUATE      → Scorer.score()          How relevant/important is each result?
3. ATTEND        → Router.select()         What matters most right now?
4. INTEGRATE     → Composer.compose()      Build the context window under budget
5. ACT           → Agent.execute()         Call LLM, produce output
6. VERIFY        → Gate.verify()           Did it work? (external truth)
7. PERSIST       → Substrate.put()         Store output with lineage (audit DAG)
8. ADAPT         → Policy.decide()         What patterns emerged?
9. META-COGNIZE  → Daimon.assess()         Am I doing this well?
```

Three issues:

1. **Step 1 is three things pretending to be one.** The runtime
   perceives via Substrate *and* via the Bus *and* via external I/O.
   Flattening these into "Substrate.query" misdescribes the system.
2. **Steps 2 and 3 are the same step.** Doc 09 §2.2 itself admits
   "in the current `loop_tick` implementation, scoring is implicit —
   the Router uses scoring internally." Evaluation and selection
   collapse into one operation: score-and-route.
3. **Step 9 is not a loop step.** `docs/00-architecture/13-cognitive-cross-cuts.md`
   describes Daimon as a cross-cut injected across multiple layers, not
   as a sequential step. Putting it inside the loop as step 9
   contradicts the cross-cut framing.

## 2. The revised seven-step loop

```
1. SENSE         Substrate.query | Bus.subscribe | external I/O
2. ASSESS        Scorer.score + Router.select   (joint: pick one, know why)
3. COMPOSE       Composer.compose → Engram (prompt)
4. ACT           execute (LLM, tool, chain call) → stream of Pulses
5. VERIFY        Gate.verify | Gate.verify_stream → Verdict Engram
6. PERSIST       Substrate.put                (Engrams only)
   BROADCAST     Bus.publish                  (Pulses, in parallel to persist)
7. REACT         Policy.decide → more Pulses + Engrams
```

Step 6 is two co-equal operations (persist durable things, broadcast
ephemeral things) that happen together, not in sequence.

Cross-cuts (Neuro, Daimon, Dreams) are **not** loop steps. They are
injected into specific steps:

- **Neuro** contributes Engrams to step 1 (knowledge retrieval) and
  step 3 (prompt enrichment), and consumes verdicts from step 5.
- **Daimon** biases step 2 (affect-modulated routing) and gates step 4
  (behavioral-state transitions suppress or enable actions).
- **Dreams** runs on its own Delta-speed loop, consuming recent
  Engrams via Substrate and emitting consolidated ones back.

## 3. Step-by-step

### Step 1 — SENSE

The runtime has three sensing primitives:

**Substrate.query** — pull Engrams by filter. Used for durable
context: recent episodes, stored plans, knowledge entries, historical
gate verdicts, cached tool results.

**Bus.subscribe** — listen for Pulses on topics of interest. Used for
live context: agent turn outputs in flight, incoming HTTP webhooks,
approval requests, cancellation, clock ticks.

**External I/O** — read from sources that aren't yet either fabric:
the LLM's WebSocket, a subprocess stdout, a filesystem watch, an
incoming HTTP request on `roko-serve`. These immediately become Pulses
(published to the Bus) and often graduate to Engrams.

A step-1 implementation is typically a `select!` over one or more
Bus receivers plus a periodic Substrate poll. The current `loop_tick`
in `crates/roko-core/src/loop_tick.rs` is a Substrate-only version of
this — it will generalize.

### Step 2 — ASSESS

Combined score + route. The input is a slice of `Datum` (Engram or
Pulse). The Scorer computes a multi-axis Score; the Router uses the
scores (plus ctx-aware logic like cascade model selection or LinUCB
bandit draws) to produce a `Selection` identifying the chosen item and
its confidence. For many loops there's only one candidate and
selection is degenerate.

The `RouterFeedback` path (`feedback(&Outcome)`) runs later, after
verify, closing the bandit learning loop.

### Step 3 — COMPOSE

The Composer takes the selected `Datum`(s) plus any additional
context (system prompt layers, tool descriptions, recent episodes) and
assembles a composed Engram under a Budget. The composed Engram is
typically `Kind::Prompt`. This is the step where token-budget
awareness lives (per design principle P3 in doc 17).

For non-LLM actions (e.g. a direct chain call, a filesystem op) the
composed Engram describes the action fully enough for step 4 to
execute it without further context.

### Step 4 — ACT

The runtime executes the action described by the Engram from step 3.
In the most common case this is an LLM call: publish `agent.process.spawned`,
stream `agent.msg.chunk` Pulses as tokens arrive, publish
`agent.turn.completed` when done. The final `AgentOutput` graduates to
an Engram for step 5 to verify.

Other act paths:

- Tool call → `tool.call.started` / `tool.call.completed` Pulses;
  `ToolInvocation` Engram.
- Chain transaction → `chain.tx.submitted` / `chain.tx.confirmed`
  Pulses; `Transaction` Engram.
- Filesystem op → `fs.op.completed` Pulse; `FsOp` Engram if auditable.

### Step 5 — VERIFY

The Gate pipeline verifies the step-4 Engram. The pipeline itself is
a Composer-composition-of-Gates specified in `roko-gate`. Each Gate
emits a `GateVerdict` Engram; the pipeline's output is an aggregate
Verdict that is itself an Engram.

**Stream-gates run in parallel** to the Engram-gates. A BudgetGate
watches `agent.tokens.used` Pulses during step 4 and can halt the
step before completion if the budget trips.

### Step 6 — PERSIST & BROADCAST (co-equal)

The step-5 Verdict Engram lands in the Substrate (lineage captured,
audit DAG updated). The same event is broadcast as a
`gate.verdict.emitted` Pulse for subscribers that care about the
live delivery.

The composed Engram from step 3 also persists here if the caller
wants it (useful for prompt replay; can be gated by config since
prompts can be large).

### Step 7 — REACT

Policies subscribed to the relevant topics receive the new Pulses
and decide. Typical reactions:

- `EpisodePolicy` sees `gate.verdict.emitted`, accumulates a turn's
  worth of Pulses, graduates an `Episode` Engram.
- `CircuitBreakerPolicy` sees a streak of failed verdicts, publishes
  `conductor.circuit.tripped`.
- `EfficiencyPolicy` sees token-usage Pulses, updates cascade-router
  feedback for the Router in step 2 of the *next* tick.
- `PlanRevisionPolicy` (the P0 self-hosting closure — item 89 in
  `tmp/ux-followup/15-safety-and-learning-closure.md`) sees N
  consecutive failure verdicts on the same task, publishes
  `plan.revision.requested`, which the orchestrator subscribes to
  and turns into a new `roko prd plan` invocation.

## 4. Three speeds, same loop

The three cognitive speeds (Gamma ≈5–15 s, Theta ≈75 s, Delta ≈ hours)
are frequencies at which the loop ticks, not different loops. Every
speed runs these seven steps; what differs is:

- **Sense scope**: Gamma senses only the last few seconds of Pulses +
  hottest Engrams. Theta senses minutes of recent Pulses + recent
  Engrams. Delta senses hours of Engrams (Pulses are long gone from
  the ring).
- **Compose budget**: Gamma gets tight token budgets. Delta can
  afford long contexts (used by Dreams consolidation).
- **Gate pipeline depth**: Gamma uses quick gates (compile only).
  Theta runs more (compile + test). Delta runs deep pipelines
  (compile + test + clippy + property tests + judge).
- **Persist cadence**: Gamma persists selectively (only graduated
  Pulses and final turns). Delta persists consolidated summaries.

## 5. Three speeds, one orchestrator

The current `PlanRunner` in `crates/roko-cli/src/orchestrate.rs` runs
what is essentially a Theta-speed loop. Gamma runs inside individual
agents (the token-streaming loop). Delta runs (will run) inside
`roko-dreams`. All three should be expressible as the same seven-step
loop at different frequencies. This is the promise of the "universal"
in "universal cognitive loop" — and it's easier to keep that promise
with Pulse and Bus in the vocabulary, because Gamma and Delta have
very different persistence costs.

## 6. What the revised doc 09 looks like

`docs/00-architecture/09-universal-cognitive-loop.md` rewrites as:

- §1 The Seven-Step Loop (replaces "nine-step")
- §2 Step 1 — SENSE (three sense sources)
- §3 Step 2 — ASSESS (joint score+route)
- §4 Step 3 — COMPOSE (prompt assembly under budget)
- §5 Step 4 — ACT (LLM / tool / chain)
- §6 Step 5 — VERIFY (Engram-gates + stream-gates)
- §7 Step 6 — PERSIST & BROADCAST (co-equal)
- §8 Step 7 — REACT (Policy.decide → Pulses + Engrams)
- §9 Cross-cuts inject into specific steps (not a ninth step)
- §10 The loop at three speeds
- §11 Shipping code: `loop_tick` with Bus integration

`crates/roko-core/src/loop_tick.rs` grows a `TickConfig` with optional
Bus subscriptions; the existing Substrate-only path is preserved as
the default for backward compat.

## 7. The self-hosting closure becomes a one-liner

`CLAUDE.md` item 11:

> Feedback loop → failed task gates feed back into plan generator for
> re-planning.

With the revised loop this is: `PlanRevisionPolicy` subscribes to
`gate.verdict.emitted`, counts failures by task hash, publishes
`plan.revision.requested` after N consecutive failures. The
orchestrator subscribes to that topic and invokes `roko prd plan`
with the failure context in its prompt. Done.

CLAUDE.md item 10:

> Automatic plan generation → trigger `prd plan` automatically when a
> PRD is published.

With the revised loop: `PrdPublishPolicy` subscribes to
`prd.published`, publishes `plan.generation.requested`. Orchestrator
subscribes. Done.

Both P0 self-hosting blockers dissolve into Bus-topic plumbing. The
only reason they looked hard before was that the Bus wasn't in the
architectural lexicon, so every proposal to "emit an event" had to
re-invent the event channel.

## 8. The `loop_tick` function after the refactor

`crates/roko-core/src/loop_tick.rs` exists today as a Substrate-only
tick. The revised shape supports a Bus subscription alongside it and
keeps the old signature for backward compat:

```rust
pub struct TickConfig<'a> {
    pub substrate: &'a dyn Substrate,
    pub bus: Option<&'a dyn Bus>,                 // optional until callers opt in
    pub scorer: &'a dyn Scorer,
    pub router: &'a dyn Router,
    pub composer: &'a dyn Composer,
    pub policies: &'a [Box<dyn Policy>],
    pub ctx: &'a Context,
    pub budget: &'a Budget,
}

pub async fn loop_tick<'a>(cfg: TickConfig<'a>) -> Result<TickOutcome> {
    // Step 1: SENSE
    let engrams = cfg.substrate.query(cfg.ctx.sense_predicate()).await?;
    let pulses = if let Some(bus) = cfg.bus {
        drain_bus_since(bus, cfg.ctx.last_seq).await?
    } else {
        Vec::new()
    };
    let data: Vec<Datum> = engrams.iter().map(Into::into)
        .chain(pulses.iter().map(Into::into))
        .collect();

    // Step 2: ASSESS
    let selection = cfg.router.select(&data, cfg.ctx)?;
    let picked = selection.as_ref().map(|s| &data[s.index]);

    // Step 3: COMPOSE
    let composed = cfg.composer.compose(picked.into_iter().collect::<Vec<_>>().as_slice(),
                                        cfg.budget, cfg.scorer, cfg.ctx)?;

    // Step 4: ACT — returns an action stream (Pulses) + final Engram
    let outcome = execute(&composed, cfg.bus, cfg.ctx).await?;

    // Step 5: VERIFY
    let verdict = cfg.ctx.gate_pipeline().verify(&outcome.engram, cfg.ctx).await?;

    // Step 6: PERSIST & BROADCAST (co-equal)
    cfg.substrate.put(outcome.engram.clone()).await?;
    cfg.substrate.put(verdict.engram.clone()).await?;
    if let Some(bus) = cfg.bus {
        bus.publish(verdict.to_pulse(Topic::new("gate.verdict.emitted"), 0,
                                     cfg.ctx.source())).await?;
    }

    // Step 7: REACT
    for policy in cfg.policies {
        let out = policy.decide(&pulses, cfg.ctx);
        for p in out.pulses { if let Some(bus) = cfg.bus { bus.publish(p).await?; } }
        for e in out.engrams { cfg.substrate.put(e).await?; }
    }

    Ok(TickOutcome { selection, composed, outcome, verdict })
}
```

About 40 lines. The signature is backward-compatible with the current
Substrate-only tick because `bus: Option`. The refactor in
`06-refactoring-plan.md` walks the tick to fully-Bus-aware over Phase
B and C, removing the `Option` when every caller has migrated.

## 9. Cross-cuts are injected, not sequenced

The nine-step loop's step 9 (META-COGNIZE via Daimon) is one of three
cognitive cross-cuts documented in
`docs/00-architecture/13-cognitive-cross-cuts.md`: Neuro, Daimon,
Dreams. Cross-cuts are not loop steps. They inject into the operators
themselves:

- **Neuro** is a `Composer` enrichment hook at step 3 and a
  tier-progression Policy at step 7. It also reads from Substrate at
  step 1 via the same `query_similar` HDC extension as everything else
  (see `11-hyperdimensional-substrate.md`).
- **Daimon** is a Scorer bias at step 2 (affect-modulated routing)
  and a Gate at step 4 (behavioral-state gating — "Daimon's mood is
  exhausted, defer this risky action"). It subscribes to
  `gate.verdict.emitted` and `agent.turn.completed` Pulses to update
  PAD.
- **Dreams** runs its own Delta-speed loop. It consumes recent
  Engrams via Substrate scan and emits consolidated Engrams (plus
  `engram.promoted` Pulses). It doesn't share a tick with the main
  loop.

Putting them inside the main loop as sequential steps was
architecturally inconsistent with the cross-cut concept doc 13
introduced. The revised seven-step model keeps the loop lean and the
cross-cuts explicit.
