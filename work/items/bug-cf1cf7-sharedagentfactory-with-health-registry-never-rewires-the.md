+++
id = "bug-cf1cf7"
kind = "bug"
title = "SharedAgentFactory::with_health_registry never rewires the already-built ModelRouter, so routing never sees a persisted registry's open circuits"
status = "open"
triage = "verified"
severity = "p2"
goal = "truth"
size = "M"
subsystem = ["roko-cli/dispatch"]
created = 2026-10-03
updated = 2026-10-03
last_verified = 2026-10-03
source = "wave-4 follow-up reports 2026-10-02 (PK02)"
discovered_from = "gap-e00238 (PK02's own package; this wiring gap is a distinct finding, not in its scope)"
anchors = ["crates/roko-cli/src/dispatch/factory.rs::with_health_registry", "crates/roko-cli/src/dispatch/model_routing.rs::ModelRouter", "crates/roko-cli/src/graph_execution/plan_runner.rs"]
lane = "rust-hot"
links = { depends_on = [], blocks = [], related = [], supersedes = [], duplicate_of = "" }

[[verify]]
command = "grep -rqw 'fn with_health_registry_rewires_the_router' crates/roko-cli/ && cargo test -p roko-cli with_health_registry_rewires_the_router"
+++

## Problem

`SharedAgentFactory::with_health_registry` (`crates/roko-cli/src/dispatch/factory.rs:298`) only replaces the
factory's own top-level field:

```rust
pub fn with_health_registry(mut self, registry: Arc<ProviderHealthRegistry>) -> Self {
    self.health_registry = registry;
    self
}
```

But the factory's internal `ModelRouter` (routing decisions) was already wired to a **different**,
freshly-created, empty registry during `SharedAgentFactory::new()` (`factory.rs`, ~line 187-191: "Callers can
replace this with a persisted workspace registry via `with_health_registry`... Keeping construction local
preserves the factory's use in path-free unit tests" — `let health_registry = Arc::new(ProviderHealthRegistry::new());`
followed immediately by `dispatcher.with_provider_health(Arc::clone(&health_registry), model_providers)`, which
wires the ModelRouter). `with_health_registry` never re-wires `self.dispatcher`'s router with the new registry.

The real run's actual call site, `crates/roko-cli/src/graph_execution/plan_runner.rs` (~line 1247-1265), confirms
the gap in practice: it loads the **persisted** `provider-health.json` (`ProviderHealthRegistry::load_or_new`),
then calls `SharedAgentFactory::new(...).await.with_health_registry(Arc::new(health_registry))` — by this point the
factory (and its already-constructed `ModelRouter`) exists and is wired to the empty registry from `new()`; the
`with_health_registry` call afterward only swaps the factory's own `self.health_registry` field (used for
recording new outcomes, e.g. by `failover.rs`), leaving the router's `health: Option<Arc<ProviderHealthRegistry>>`
(`crates/roko-cli/src/dispatch/model_routing.rs:311`) pointed at the original empty one.

## Why it matters

Goal: truth / golden-path reliability. A provider whose circuit was left open at the end of a previous run (saved
to `provider-health.json`) is correctly excluded from **recording** new outcomes against a fresh, blank slate, but
is never excluded from **routing** at the start of the next run — `ModelRouter::route` (model_routing.rs:530)
consults its own `health` field, which still shows every provider healthy, regardless of what was persisted. A
provider known to be in a bad state (still exhausted, still rate-limited) can be routed to again immediately at the
start of a new run, until it fails again in-process and re-trips its (now again fresh) circuit — defeating the
purpose of persisting health across runs at all.

## Where

- `crates/roko-cli/src/dispatch/factory.rs::SharedAgentFactory::new` (~line 187-191): constructs a fresh, empty
  `ProviderHealthRegistry` and wires it into the dispatcher's `ModelRouter` via `with_provider_health` before the
  caller ever gets a chance to supply a persisted one.
- `crates/roko-cli/src/dispatch/factory.rs::SharedAgentFactory::with_health_registry` (line 298): only sets
  `self.health_registry`, never touches `self.dispatcher`'s router.
- `crates/roko-cli/src/dispatch/model_routing.rs::ModelRouter` (`health: Option<Arc<ProviderHealthRegistry>>`,
  line 311; `with_provider_health`, used at construction) — the field that stays stale.
- `crates/roko-cli/src/graph_execution/plan_runner.rs` (~line 1247-1265) — the real call site: loads the persisted
  registry, then calls `.with_health_registry(...)` on an already-constructed factory, expecting it to take effect
  for routing too.

## Current state

Unfixed. Every code path that actually runs a plan goes through `plan_runner.rs`'s sequence above, so this affects
every real run with a persisted `provider-health.json`, not just an edge case.

## Plan

1. Give `SharedAgentFactory::with_health_registry` the ability to also re-wire the already-built `ModelRouter`:
   either expose a `self.dispatcher.replace_provider_health(registry)`-style method on the dispatcher/router (the
   dispatcher already has `replace_routing_ladder`, `without_models` for an analogous late-rewire pattern, per
   `factory.rs` around the `with_disabled_models` method), or restructure construction so the caller can supply the
   health registry to `SharedAgentFactory::new(...)` itself, before the router is built, rather than overriding it
   afterward.
2. Test: construct a `SharedAgentFactory`, call `with_health_registry` with a registry that already has an open
   circuit for provider X, then call the router's `route()` and confirm it excludes X — today it would not.

## Done when

- A `SharedAgentFactory` built with `with_health_registry(registry)`, where `registry` already shows an open
  circuit for a provider, routes away from that provider on the very first call of the new process.
- The `[[verify]]` command passes.

## Notes

- Do not confuse this with `with_disabled_models`/`replace_routing_ladder` (a different, config-driven exclusion
  mechanism already wired correctly) — this item is specifically about the live, persisted circuit-breaker state.
