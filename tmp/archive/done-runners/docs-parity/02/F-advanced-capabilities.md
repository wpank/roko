# F — Advanced Capabilities

Refresh target: `docs/02-agents/12-extensibility.md` and the advanced sections of `00-agent-trait.md`

Verdict: `defer`

---

## Current Parity Summary

| Topic | Current state | Notes |
|---|---|---|
| Extension points | Shipping | `Agent`, `ProviderAdapter`, `Translator`, `LlmBackend`, and `ToolHandler` are all real |
| Advanced primitives | Shipping | `CompositeAgent`, `MorphableAgent`, `MetacognitiveMonitor`, plugin/event traits, and `ProcessSupervisor` exist |
| Domain profiles | Deferred | the doc surface is ahead of code and usage |
| Plugin SPI tiers 4-5 | Deferred | too speculative for current parity copy |

---

## What Is Already Real

Keep these in the parity story:

- `CompositeAgent`
- `MorphableAgent`
- `MetacognitiveMonitor`
- `EventSource`
- `FeedbackCollector`
- `PluginManifest`
- `ProcessSupervisor`

These are legitimate advanced surfaces because the code exists today.

---

## What Must Be Marked As Deferred

### Domain profiles

The domain-profile narrative should move out of present tense.

Use wording like:

- target-state
- deferred
- not yet a live runtime product surface

Do not present these as shipped:

- six canonical deployment profiles as an operationalized product surface
- installable domain bundles
- typed domain context packages as current agent-runtime requirements

### Plugin SPI tiers 4-5

These belong in future work, not present-tense parity copy.

Why:

- no external plugin-author pressure is evident here
- the parity blocker is not missing SPI tiering
- the audit explicitly called this overreach out

### Research-heavy self-evolving systems

Keep out of the live parity story:

- shared agent memory
- archive systems
- Darwin/Godel-style self-evolving loops
- other moat-style claims not backed by current runtime use

---

## Recommended Refresh Language

- Keep: real extension points and real advanced primitives.
- Rewrite: advanced sections that currently blur “crate exists” with “platform feature is operationalized.”
- Defer: domain profiles, plugin SPI tiers 4-5, and research-heavy agent-evolution systems.

---

## Verification Anchors

```bash
rg -n "pub struct CompositeAgent|impl Agent for CompositeAgent" crates/roko-agent/src/composition.rs
rg -n "pub struct MorphableAgent|impl Agent for MorphableAgent" crates/roko-agent/src/metamorphosis.rs
rg -n "pub trait EventSource|pub trait FeedbackCollector|pub struct PluginManifest" crates/roko-plugin/src/lib.rs
rg -n "pub struct ProcessSupervisor" crates/roko-runtime/src/process.rs
rg -n "domain profiles|TypedContext|Custody|installable bundles" docs/02-agents/16-domain-profiles.md docs/02-agents/INDEX.md
```
