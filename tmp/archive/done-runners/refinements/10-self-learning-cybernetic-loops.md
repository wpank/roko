# Self-Learning & Cybernetic Feedback Loops

> **TL;DR**: Once the Bus exists as a first-class fabric, *every operator
> becomes a learner*, not just the three that already happen to be
> (CascadeRouter's bandit, EpisodePolicy's replay, experiments' A/B). The
> Free Energy Principle stops being a metaphor and becomes a concrete
> variational-inference loop: operators predict, publish predictions as
> Pulses, get corrected by later Pulses, and update. Roko becomes a
> self-modeling system whose prediction error is a first-class signal.

> **For first-time readers**: Roko already has three partial learners —
> `CascadeRouter` (model bandit), `EpisodeLogger` (turn replay), and
> `ExperimentStore` (prompt A/B). The six kernel operators (Scorer, Gate,
> Router, Composer, Policy + Substrate/Bus) were not designed as learners.
> This doc argues they *already are*, latent — and that the Pulse/Bus
> primitives from 02–03 make the predict-correct-update loop uniform across
> all of them. The payoff: active inference (Friston) becomes an
> implementation technique, not a metaphor. Read 02–05 for vocabulary; read
> 11 and 14 alongside this one for the HDC and heuristic substrates that
> make individual operator-level calibration stick.

## 1. The one thing the current design gets wrong about learning

Today, "learning" in Roko is three things stapled on the side:

1. **`CascadeRouter` bandit** — chooses a model per turn and updates from
   reward (`roko-learn/src/cascade_router.rs`).
2. **Episode replay** — stores full turns in `.roko/episodes.jsonl` and
   periodically distils them into playbooks (`roko-learn/src/episode.rs`).
3. **Prompt A/B experiments** — `ExperimentStore` picks among variants and
   tracks win-rate (`roko-learn/src/experiments.rs`).

Everything else — Scorer weights, Gate thresholds, Router topologies
beyond cascade, Composer template selection, Policy parameters — is
hardcoded or configured, never learned.

The pattern that should emerge naturally from the two-fabric refactor:
**every operator is a predictor; every downstream Pulse is a potential
training signal**. The Bus is the universal feedback channel.

## 2. Active inference as a literal implementation

### 2.1 The Free Energy Principle in one paragraph

Friston's FEP says an agent maintains a generative model of its world,
makes predictions that minimize *expected* free energy (prediction error
+ complexity penalty), acts to make its predictions true, and updates
the generative model when they fail. Most AI implementations treat it
as a metaphor. With Bus + Pulse, it can be literal.

### 2.2 The predict-publish-correct loop

For any operator `O` that produces output `y` from input `x`:

```text
1. O.predict(x) publishes Pulse{topic = "O.prediction", body = y_hat, lineage_hint = x.hash}
2. Downstream reality publishes Pulse{topic = "O.outcome", body = y_true, lineage_hint = x.hash}
3. A learning policy subscribes to both, joins by lineage_hint, and
   publishes Pulse{topic = "O.error", body = (y_hat, y_true, loss)}
4. O subscribes to its own error topic and updates internal state.
```

This is active inference in four bullets. It runs on top of the Bus
trait; no new primitive needed.

### 2.3 Concrete example: Scorer calibration

The Scorer assigns a 7-axis Score to every candidate. Today, Score axes
are tuned by hand. With active inference:

- Scorer publishes `scorer.prediction` Pulses containing
  `(engram_hash, predicted_reward)`.
- The GateVerdict stream publishes `gate.verdict.emitted` with
  `(engram_hash, success)`.
- A `ScorerCalibrationPolicy` subscribes to both, builds empirical
  calibration curves per-axis, and publishes `scorer.weights.updated`
  Pulses.
- Scorer subscribes to `scorer.weights.updated` and reloads weights.

Nothing else in the system changes. The Scorer became self-calibrating.
Every axis now has an empirical reliability curve. Humans can inspect
it in the TUI (F4 Learn tab already exists).

### 2.4 Per-operator calibration is the breakthrough

Most "learning" in agent systems is at two extremes: weights inside the
LLM (pre-training; we don't touch) and bandit over models (CascadeRouter;
we do, but only that). Per-operator calibration is the missing middle
tier. With the Bus, every operator gets one essentially for free.

| Operator | Predicts | Outcome signal | Update policy |
|---|---|---|---|
| Scorer | 7-axis reward | Gate verdict + episode reward | Online least-squares on each axis |
| Router | selected action will succeed | Gate verdict | Contextual bandit (already in cascade; generalize) |
| Composer | prompt will fit budget + win gate | Token count + gate verdict | Template EMA; template bandit |
| Gate | task will succeed post-patch | Next gate verdict + regression tests | Threshold EMA (already partial in `adaptive.rs`) |
| Policy | decision will improve metric | Metric Pulse after decision | Per-policy online calibration |
| Substrate | Engram tier is correct | Query frequency + recency | Tier-promotion Markov chain |

Six self-calibrating operators instead of three partially-adaptive
subsystems. Scale: every call in the system produces training data for
something.

## 3. Closed-loop prompt optimization (DSPy-style, but native)

Stanford's DSPy project (Khattab et al. 2023) compiles prompts rather
than writing them: you describe a program in modules, provide a metric,
and DSPy optimizes prompts by bootstrapping examples and A/B testing.
Roko's Composer plus the Bus gives us a tighter, native version:

### 3.1 The Composer as a compiler target

Composer templates become first-class `TemplateEngram`s (stored,
versioned, content-addressed). Each template has a vacant "slot" for
the input Engram and a `SuccessMetric` field linking to a Gate
pipeline whose verdict is the template's reward.

### 3.2 The optimization loop

```text
1. ExperimentPolicy publishes Pulse{topic = "template.variant.proposed",
   body = TemplateEngram'} containing a mutation of an existing template
   (rewrite an instruction, add an example, shorten a preamble).
2. The Composer, under a feature flag, routes N% of traffic to the new
   variant via an A/B split (exists today in ExperimentStore).
3. Gate verdicts land on the Bus; the ExperimentPolicy accumulates
   wins/losses by template hash.
4. After M trials, ExperimentPolicy publishes Pulse{topic = "template.promoted"}
   if the variant beats control by epsilon; otherwise "template.rejected".
5. The Dreams consolidation loop (Phase 5C) subscribes to "template.promoted"
   Pulses across agents and distils the winners into a meta-template
   manifesto that mutation policies draw from.
```

### 3.3 The meta-level: mutation policies are themselves learned

The mutation itself (rewrite, add example, shorten) is chosen by a
`MutationPolicy` that's itself learning what kinds of mutations tend to
work on what kinds of templates. The Bus topic `mutation.outcome`
carries (mutation_type, template_hash, won_ab). Over time, the
MutationPolicy becomes a prompt-evolution genetic algorithm whose
fitness function is the Gate.

### 3.4 Why this is better than DSPy

DSPy runs offline compilation passes on human-chosen metrics. Roko's
version runs *continuously* during normal operation. Every production
turn is a training sample. DSPy is batch; Roko is online.

## 4. Cybernetic feedback hierarchy

Stafford Beer's Viable System Model describes five recursive systems,
each a feedback loop over the one below. Roko's three speeds map
cleanly:

- **Gamma (5–15 s)**: Beer's System 1 — operations. Individual agent
  turns, token streams, immediate gate decisions.
- **Theta (~75 s)**: Beer's System 2+3 — coordination and internal
  regulation. The orchestrator's plan-level decisions, circuit breakers,
  efficiency policies.
- **Delta (hours)**: Beer's System 4+5 — intelligence and identity.
  Dreams consolidation, Neuro tier progression, PRD-level revision,
  meta-template optimization.

Each speed has its own Bus topic namespace (`gamma.*`, `theta.*`,
`delta.*`). Cross-speed Pulses are explicit — the orchestrator at
Theta publishes `delta.plan.revision.requested` when accumulated Gamma
errors exceed threshold. This is Beer's *algedonic signal*: a
cross-layer alarm that bypasses the normal hierarchy when the lower
layer is failing. The Pulse model makes algedonic signals trivial.

## 5. Prediction error as a first-class metric

### 5.1 The `prediction_error` axis

Add a seventh Pulse topic family: `prediction.error.*`. Every predictor
publishes to it. Every learner subscribes. The TUI F4 tab grows a
"Prediction Error" sub-view showing:

- Per-operator calibration curves (predicted vs actual)
- EMA of prediction-error magnitude per topic
- Drift detection (sudden spike = model of world broken; trigger Dreams)

### 5.2 Prediction-error drives attention

High-prediction-error regions of the Substrate are where the agent is
*learning most*. Dreams should prioritize them for consolidation;
Neuro should promote their Engrams to higher tiers; the orchestrator
should re-plan around them. The Bus makes this a one-liner: subscribe
to `prediction.error.high`, enqueue the lineage for deeper analysis.

This is the formal implementation of curiosity-driven learning (Oudeyer
& Kaplan 2007) and intrinsic motivation (Schmidhuber 2010): agents
preferentially attend to regions where their models are improvable.

### 5.3 Prediction error becomes the c-factor anchor

Collective prediction error — summed across all operators in a
collective — is a sharper c-factor proxy than any individual metric.
Groups that collectively predict better are more intelligent. This is
operationally measurable at every tick. (Expanded in doc 15.)

## 6. Exponentially scaling loops

Cybernetics defines a positive feedback loop by its *gain*: output that
amplifies its own input. Roko has three natural exponential loops
available once the Bus is first-class:

### 6.1 Agents teaching agents (meta-imitation)

When agent A's turn wins a hard gate and agent B's turn loses on the
same task, the Bus has:

- `agent.turn.completed` from A with verdict=pass
- `agent.turn.completed` from B with verdict=fail
- Both with the same parent task hash

A `CrossAgentLearningPolicy` subscribes, extracts (A's prompt + A's
trajectory, B's prompt + B's trajectory), stores the pair as a
`ContrastiveEngram`, and feeds it into the next round of
Composer-template mutation. B literally learns from A without either
agent noticing.

At N agents, this is N(N-1) pairwise learning streams. Exponential
surface area.

### 6.2 Playbooks-of-playbooks

Today: playbooks distil episodes into reusable patterns. Next tier:
**meta-playbooks** distil playbooks into strategies ("when the task is
X, use playbook P, but if the first gate fails try P'"). Next: **plays**
distil meta-playbooks into policies. Every tier is a Delta-speed
consolidation loop over the previous tier's output topic. The data
volume compresses by ~10x per tier; knowledge depth grows
exponentially.

### 6.3 Self-modeling and meta-Gate

The agent maintains a model of itself: "what kinds of tasks does this
agent succeed on; what's its current prediction-error trend". Periodically,
a `MetaGate` runs on the agent's model of itself: if the model's own
predictions about itself are wrong, trigger a deeper review. This is
second-order feedback: the agent's learning about its own learning.

Second-order feedback enables Minsky's "society of mind" at the
architectural layer: each agent is an "A-brain", and the MetaGate is
the "B-brain" watching it. (Minsky 1986, §6.)

## 7. Making the current learning subsystems read off the Bus

The existing three learners in `roko-learn` become trivial after the
refactor:

### 7.1 CascadeRouter (already a bandit)

Today it's called explicitly by the router. After: it subscribes to
`router.selection.made` and `router.selection.outcome`, updates its
internal bandit, publishes `router.weights.updated`. Same mechanism as
everything else. Decouples from the Router code.

### 7.2 EpisodeLogger

Today: called from orchestrate.rs. After: subscribes to
`agent.turn.completed` + `gate.verdict.emitted`, constructs Episodes
by correlating lineage, stores them. Orchestrator doesn't know it
exists. Fully decoupled.

### 7.3 ExperimentStore

Today: called from Composer when building prompts. After: subscribes to
`composer.invocation.started`, decides variant, publishes
`composer.variant.assigned`. The Composer subscribes back for its
template choice. Hit rates tracked by listening to Gate verdicts.

None of this is *necessary* before Phase C; the current wiring works.
But the refactored version makes it much easier to add the fourth,
fifth, sixth learner without more crate dependencies.

## 8. The meta-insight

Once the Bus exists, **any function of past Pulses is a learnable
signal**. This is a superset of almost everything in the RL/LLM
training literature, because:

- Supervised learning: subscribe to `(input, label)` Pulses.
- RL: subscribe to `(state, action, reward)` Pulses.
- Imitation: subscribe to `(expert_action)` Pulses.
- Self-play: two agents publish on separate topics; a Policy joins them.
- Curiosity: subscribe to `prediction.error.*`.
- Distillation: subscribe to big-model outputs, train small-model
  responses.

All of these collapse to the same primitive: *subscribe to topics,
build a policy, publish new Pulses*. The framework becomes the learning
algorithm, not a substrate that learning happens on top of.

## 9. Practical next step (Phase C.5)

After Phase C's subsystem migration, a two-week Phase C.5 would:

1. Add `prediction.*` and `outcome.*` topic families to
   `roko-core::topics`.
2. Wrap each of the six operators with a thin "record prediction"
   instrument that publishes on the prediction topic without changing
   the operator's signature.
3. Land a single `CalibrationPolicy` in `roko-learn` that subscribes to
   every prediction/outcome pair, maintains per-operator calibration
   state, and publishes updates.
4. Connect the TUI F4 tab to render live calibration.

After that, the system has a *complete* feedback nervous system. Every
subsequent learning feature is a new subscription.

## 10. The `CalibrationPolicy` sketch

The Phase-C.5 work collapses to roughly this structure:

```rust
// crates/roko-learn/src/calibration/mod.rs (new)
use roko_core::{Bus, Pulse, Topic, TopicFilter};
use std::collections::HashMap;

/// Per-operator calibration state, indexed by operator name.
#[derive(Default)]
pub struct CalibrationState {
    pub trials: u64,
    pub squared_error_sum: f64,       // for RMSE
    pub brier_sum: f64,               // for probabilistic predictions
    pub ema_error: f64,               // exponential moving avg
    /// Per-axis calibration curve bins (for Scorer's 7-axis case).
    pub axis_curves: Vec<CalibrationBin>,
}

pub struct CalibrationBin {
    pub predicted: f64,
    pub observed: f64,
    pub count: u64,
}

/// A single Policy that watches all predict/outcome pairs and
/// publishes calibration updates per operator.
pub struct CalibrationPolicy<B: Bus> {
    pub bus: std::sync::Arc<B>,
    /// Map from (operator_name, lineage_hint) → predicted value.
    /// Outcome Pulses close the loop by matching lineage_hint.
    pending: parking_lot::Mutex<HashMap<(String, ContentHash), PredPayload>>,
    state: parking_lot::Mutex<HashMap<String, CalibrationState>>,
    ema_alpha: f64,  // e.g. 0.02
}

impl<B: Bus> CalibrationPolicy<B> {
    pub async fn run(self: std::sync::Arc<Self>) {
        let filter = TopicFilter::Or(
            Box::new(TopicFilter::Glob("prediction.**".into())),
            Box::new(TopicFilter::Glob("outcome.**".into())),
        );
        let mut rx = self.bus.subscribe(filter).await.unwrap();
        while let Some(pulse) = rx.recv().await {
            if pulse.topic.as_str().starts_with("prediction.") {
                self.record_prediction(&pulse);
            } else if pulse.topic.as_str().starts_with("outcome.") {
                if let Some(update) = self.close_and_update(&pulse) {
                    let _ = self.bus.publish(self.emit_update(update)).await;
                }
            }
        }
    }

    fn record_prediction(&self, p: &Pulse) {
        let Some(operator) = p.source.component.strip_prefix("operator:") else { return };
        let Some(lineage) = p.lineage_hint.clone() else { return };
        let Some(payload) = PredPayload::from_body(&p.body) else { return };
        self.pending.lock().insert((operator.to_string(), lineage), payload);
    }

    fn close_and_update(&self, p: &Pulse) -> Option<CalibrationUpdate> {
        let operator = p.source.component.strip_prefix("operator:")?.to_string();
        let lineage = p.lineage_hint.clone()?;
        let pred = self.pending.lock().remove(&(operator.clone(), lineage))?;
        let truth = OutcomePayload::from_body(&p.body)?;
        let err = loss(&pred, &truth);
        let mut state = self.state.lock();
        let s = state.entry(operator.clone()).or_default();
        s.trials += 1;
        s.squared_error_sum += err * err;
        s.ema_error = s.ema_error * (1.0 - self.ema_alpha) + err * self.ema_alpha;
        update_axis_curves(&mut s.axis_curves, &pred, &truth);
        Some(CalibrationUpdate {
            operator,
            rmse: (s.squared_error_sum / s.trials as f64).sqrt(),
            ema_error: s.ema_error,
            trials: s.trials,
        })
    }

    fn emit_update(&self, u: CalibrationUpdate) -> Pulse {
        Pulse {
            seq: 0,
            topic: Topic::new(&format!("calibration.{}.updated", u.operator)),
            kind: roko_core::Kind::Metric,
            body: roko_core::Body::Json(serde_json::json!({
                "operator": u.operator,
                "rmse": u.rmse,
                "ema_error": u.ema_error,
                "trials": u.trials,
            })),
            emitted_at_ms: now_ms(),
            source: roko_core::PulseSource {
                component: "roko-learn:calibration".into(),
                agent_id: None,
            },
            lineage_hint: None,
            trace_id: None,
        }
    }
}
```

The operator itself just publishes predictions on a `prediction.*`
topic and listens to `calibration.<self>.updated` for weight updates.
No coupling between operators; all through the Bus. Adding a seventh
learner is writing one predict-publish and one subscribe-update pair.

## 11. Intrinsic motivation and where to send attention

Once `prediction.error.*` is a live signal stream, two concrete
behaviors become trivial to implement:

1. **Next-task scheduling biased by prediction error.** An
   `IntrinsicMotivationPolicy` subscribes to recent
   `prediction.error.*` Pulses, aggregates per-topic or per-domain,
   and publishes `attention.request` Pulses when a region's error is
   elevated. The orchestrator's plan generator honors these requests
   by prioritizing tasks that touch the high-error region.
2. **Dreams consolidation priority.** Dreams (Phase-2) wakes on
   `substrate.engram.stored` density, but it can *also* wake when
   `prediction.error.ema` crosses threshold — the regions where the
   system is confidently wrong are the regions where replay +
   consolidation pay off most.

Both are three- to five-lines-of-Rust additions. Both are direct
implementations of Oudeyer-Kaplan curiosity-driven learning and
Schmidhuber's compression-progress-as-intrinsic-reward. The runtime's
"interest" is formally where its error is highest, and that's
observable from the Bus.

## 12. Where this synergizes across the folder

Self-learning is the most pervasive primitive in the refactor — it
touches nearly every other refinement:

- **HDC (11)** makes per-operator predictions *comparable*: the
  prediction vector and the outcome vector both fingerprint into the
  same HD space, so `Similarity(fp(predicted), fp(actual))` is a
  universal error signal.
- **Demurrage (12)** is reinforced by the `ReinforceKind::Surprised`
  signal that `prediction.error.high` produces. Surprising Engrams
  stay warm longer; unsurprising ones fade.
- **Heuristics (14)** have a built-in calibration field — they are
  the highest-value case for per-operator predict/outcome pairs. A
  heuristic whose falsifier fires is exactly a prediction error.
- **c-factor (13)** treats cross-agent peer-prediction error as the
  social-perceptiveness metric. The calibration infrastructure is
  the plumbing for the c-factor measurement.
- **Replication ledger (16)** is calibration applied to paper claims
  — "we predict this paper's effect replicates; we observed Y." The
  ledger is a `CalibrationPolicy` instance specialized to paper
  lineage.
- **Competitive moat (18)** §2.1 — the *composition* of these loops
  is the moat. Any single loop is a research project; all of them
  through one Bus + Substrate is an architecture.
