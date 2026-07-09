# E — Routing And Temperament

Refresh target: `docs/02-agents/08-harness-engineering.md`, `10-temperament-profiling.md`, `11-dual-process-routing.md`

Verdict: `narrow`

---

## Current Parity Summary

| Topic | Current state | Notes |
|---|---|---|
| `ModelTier` | Shipping | stable tier surface in `roko-core` |
| `CascadeRouter` | Shipping | live router in `roko-learn`, used from CLI runtime |
| Active inference | Exists | code is present and exposed through the router, but not the default CLI dispatch path |
| Temperament | Partial | present as a human-readable string, not a typed, broadly propagated runtime policy |

---

## What Is Definitely Live

### Cascade router

`CascadeRouter` is a real runtime subsystem, not just a research sketch.

Evidence:

- definition: `crates/roko-learn/src/cascade_router.rs:994`
- active inference helper on the router: `crates/roko-learn/src/cascade_router.rs:1213`
- routed task dispatch uses the router from the CLI runtime
- runtime feedback also owns and reloads router state

This is enough to keep the routing docs and their implementation status as `Shipping`.

### Active inference code exists

`active_inference.rs` is present in `crates/roko-learn/src/active_inference.rs`.

That means the parity copy can say:

- active inference code exists
- the router has an integration point for it

It should not say:

- active inference is the default orchestrator routing path

### Harness and routing theory sections can stay, but only as support

The docs can still reference the theoretical framing.

The parity refresh should keep that framing subordinate to the live implementation:

- `CascadeRouter`
- `ModelTier`
- persistence
- route explanation
- active inference hook

---

## What Needs Narrow Wording

### Temperament is not a typed runtime contract yet

Current evidence:

- `AgentIdentity` stores `temperament: String` in `crates/roko-agent/src/introspection.rs:12`
- there is no shared typed temperament enum or broad propagation story in the current runtime path

So the refreshed docs should say:

- temperament exists as descriptive metadata
- broad policy propagation is still target-state

They should not say:

- temperament already tunes routing, gates, tool selection, and review depth across the live runtime

### Active inference is optional, not default

The helper exists, but the main CLI/orchestrator path is still the standard router path.

That matters because earlier parity notes blurred “code exists” into “runtime is using it everywhere.”

### Defer research-heavy router expansions

Keep them out of the live parity story:

- meta-router overlays
- anti-monoculture / collapse-avoidance systems
- advanced research-policy stacks that are not clearly reachable from current runtime paths

---

## Recommended Refresh Language

- Keep: `ModelTier`, `CascadeRouter`, bandit/routing infrastructure, and persistence.
- Keep but narrow: active inference.
- Rewrite: temperament sections so they clearly separate descriptive state from implemented runtime policy.
- Defer: research-heavy routing additions that do not have a visible call path.

---

## Verification Anchors

```bash
rg -n "pub enum ModelTier" crates/roko-core/src/agent.rs
rg -n "pub struct CascadeRouter|select_tier_with_active_inference" crates/roko-learn/src/cascade_router.rs
rg -n "pub struct BeliefState" crates/roko-learn/src/active_inference.rs
rg -n "temperament|pub struct AgentIdentity" crates/roko-agent/src/introspection.rs
rg -n "CascadeRouter" crates/roko-cli/src/orchestrate.rs crates/roko-cli/src/main.rs crates/roko-learn/src/runtime_feedback.rs
```
