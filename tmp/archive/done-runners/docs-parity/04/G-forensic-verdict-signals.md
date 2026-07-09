# G — Forensic Replay & Verdicts as Signals

Post-audit split between the **shipped verdict path** and the **deferred forensic system**.

---

## Verdict

Two very different stories live in this section:

- `Kind::GateVerdict` emission is **live**
- forensic replay / causal analysis is **deferred**

The old parity pack blurred them together and made the whole section read more unfinished than it really is.

---

## Shipped: Verdicts As Signals

These are current runtime truths:

- `run_gate_pipeline(...)` persists gate verdict engrams
- the orchestrator emits a conductor-side `Kind::GateVerdict` signal
- plan state stores gate results
- gate episodes feed the learning/runtime record

Key anchors:

- `crates/roko-cli/src/orchestrate.rs:12604-12732`
- `crates/roko-cli/src/orchestrate.rs:12635-12646`
- `crates/roko-cli/src/orchestrate.rs:12706-12715`
- `crates/roko-cli/src/orchestrate.rs:6181-6245`

This is enough to keep verdict-signal language in the shipped story.

---

## Narrow: Signal Contract Details

The refreshed docs should be careful about two things:

- lineage is real through `Engram::derive(...)`
- explicit 24h verdict half-life and full tag propagation are **not** obviously guaranteed by the current builder path

Key anchors:

- `crates/roko-core/src/engram.rs:131-136`
- `crates/roko-core/src/engram.rs:161-187`
- `crates/roko-core/src/decay.rs:21-30`
- `crates/roko-core/src/decay.rs:104-107`

So the right wording is:

- verdict signals are live
- the stronger contract described in the docs should be labeled as partial / target-state where code evidence is not explicit

---

## Deferred: Forensic Replay

These should move out of the shipped story:

- replay reconstruction across episodes, signals, and artifacts
- root-cause / what-if / gap analysis
- verdict clustering and trend analysis
- verdict-driven replanning
- predictive gate selection

Those are useful future ideas, but they are not prerequisites for explaining today’s verification runtime.

---

## Replacement Summary

Use this posture:

- **shipped**: `GateVerdict` production, persistence, learning/event visibility
- **partial**: stronger decay/tag/chain-verification contract
- **deferred**: forensic replay and advanced verdict analytics

That is the correct post-audit split.
