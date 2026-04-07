# Removed roko-learn Modules (2026-04-26)

Removed 8 modules (4,808 LOC) from `crates/roko-learn/src/` that had zero production callers. All code compiled and had tests but was never imported or called from any runtime path (orchestrate.rs, gateway.rs, CLI commands, or serve routes).

The modules are recoverable from git history on the `wp-arch2` branch prior to this cleanup.

## Removed Modules

### contextual_bandit.rs (1,372 lines) — RT17, superseded

Built by the roko-trustworthy RT17 batch. Sophisticated contextual bandit with feature vectors, Thompson sampling, epsilon-greedy, policy update candidates, and JSONL persistence. Never called because `UcbBandit` (in `bandits.rs`) already handles model selection in `gateway.rs`. The contextual bandit is more powerful but was a parallel implementation that was never chosen over the simpler one.

**To reintegrate:** Wire `ContextualBanditPolicy::select_action()` into `select_model_via_router()` in `roko-serve/src/routes/gateway.rs`, replacing or layering on top of `UcbBandit`. Would need feature extraction from `CompletionRequest` metadata.

### bandit_research.rs (862 lines) — doc-parity shells

Research-oriented bandit shells (Thompson sampling, Neural UCB variants) built to match learning documentation. Not production implementations — they're reference/shell code for doc parity.

**To reintegrate:** Only useful if building a research comparison harness for bandit strategies. Not needed for production routing.

### causal.rs (699 lines) — TA-08, theoretical

Granger causality tests, PC algorithm, and formal causal DAG construction from time series data. Requires a stable time-series signal pipeline that doesn't exist yet.

**To reintegrate:** Needs a signal time-series aggregation surface first (e.g., windowed episode/gate metrics). Wire into a diagnostic or analysis command like `roko learn analyze-causality`.

### shapley.rs (518 lines) — P1-08, no multi-agent credit surface

Shapley-value attribution for fair credit distribution among agents. Requires multi-agent collaboration tracking where credit assignment matters. No such surface exists in the current single-agent-per-task dispatch model.

**To reintegrate:** Relevant when tasks are decomposed across multiple agents and you need to attribute success/failure credit. Wire into post-plan-completion analysis.

### resonant_patterns.rs (373 lines) — TA-09, theoretical

Evolutionary resonant pattern organisms with Lotka-Volterra population dynamics, Price equation tracking, and HDC genomes. Research-grade evolutionary algorithm with no integration point in the current runtime.

**To reintegrate:** Would need a pattern population store and a runtime loop that evolves/selects patterns over time. Could feed into playbook rule discovery.

### kalman.rs (354 lines) — P2-10, no oracle pipeline

Kalman filter for online signal smoothing in oracle predictions. Requires an oracle prediction pipeline that doesn't exist yet (oracle predictions are built in roko-learn/oracles but not fed through a smoothing/calibration surface).

**To reintegrate:** Wire between oracle raw predictions and the prediction consumer. Useful for de-noising oracle confidence scores over time.

### adversarial.rs (321 lines) — TA-10, theoretical

HDC-based adversarial signal detection with attack prototype library. Detects adversarial/poisoned signals using hyperdimensional computing similarity. No signal validation surface currently consumes this.

**To reintegrate:** Wire into signal ingestion (roko-fs or the event subscriber) as a pre-filter that flags suspicious signals before they enter the learning pipeline.

### signal_metabolism.rs (309 lines) — TA-07, theoretical

Evolutionary signal population dynamics: replicator dynamics, Hebbian learning, and Fisher variance monitoring. Models signals as a population with birth/death/mutation dynamics. No runtime consumer.

**To reintegrate:** Would need a periodic "metabolism tick" in the runtime that evolves the signal population. Could feed into anomaly detection or signal pruning.
