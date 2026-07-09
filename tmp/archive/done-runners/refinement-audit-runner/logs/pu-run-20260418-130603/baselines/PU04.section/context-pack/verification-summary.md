# Verification Summary — Batch 04

Concise runtime picture for agents entering `04` without prior context.

## What Is Already Real

- `roko-gate` contains a substantial verification library: selector, pipeline, adaptive thresholds, feedback classifier, artifact store, ratchet, and many gate implementations.
- `orchestrate.rs` already emits `Kind::GateVerdict` signals, records gate episodes, and updates adaptive-threshold statistics.
- episodes, signals, and efficiency events already give the system the raw data foundations for later learning or replay work.

## What Is Misleading Today

- the live runtime still behaves much closer to “compile / test / clippy with raw retry output” than the docs imply,
- `select_rungs`, `GatePipeline`, `GateRatchet`, and `feedback_for_agent` are mostly library-level truth, not runtime truth,
- `AdaptiveThresholds` updates itself from real runs but does not steer real runs,
- verdict signals are emitted, but the emitted engrams do not yet match the stronger contract described in doc 15 §2.2.

## What Batch 04 Should Usually Do

1. make the canonical selector / pipeline path real,
2. make adaptive policy affect runtime behavior,
3. persist long-running verification state,
4. harden the verdict-signal contract,
5. defer reward-model, autonomous evaluator-agent, and replay-analytics work.
