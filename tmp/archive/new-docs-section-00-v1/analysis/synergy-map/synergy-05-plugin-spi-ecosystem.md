# S5 — Plugin SPI × Substrate × Bus → Ecosystem growth path

> Plugins declare what they can read from Substrate and what topics they subscribe to on the Bus.
> The SPI constrains the extension boundary so new tools, gates, roles, and domain profiles land
> without rewriting the core. Growth is structurally safe because it happens along declared seams.

**Status**: Analysis — target-state synergy  
**Primitives involved**: P10 Plugin SPI + domain profiles × P4 Substrate × P3 Bus  
**Reality check**: P4 Substrate is Shipping. P3 Bus (`EventBus<E>`) is Built. P10 Plugin SPI
and domain profiles are Scaffold. The full declared-seam ecosystem model is target-state.  
**Last reviewed**: 2026-04-19

---

## Primitives Involved

| Primitive | Role in this synergy |
|---|---|
| [P10 Plugin SPI + domain profiles](../../subsystems/) | The extension surface: a declared interface that plugins implement to register capabilities, declare Substrate reads, and subscribe to Bus topics |
| [P4 Substrate](../../reference/03-substrate/) | The storage fabric that plugins can read (and optionally write) within the bounds declared in their manifest |
| [P3 Bus / `EventBus<E>`](../../reference/04-bus/) | The transport surface through which plugins receive lifecycle events and publish their own outputs |

---

## What the Synergy Unlocks

### The core extension problem

Every growing system faces the same architectural fork: open extension (anything can hook
anything) or closed core (only blessed code can extend). Open extension is fast but fragile —
third-party hooks corrupt invariants in unpredictable ways. Closed core is safe but slow.

The synergy offers a third path: **extension along declared seams**.

A plugin does not hook arbitrary code paths. It declares:
- What Substrate data it reads (and optionally writes), expressed as a query manifest.
- Which Bus topics it subscribes to (input) and which it publishes on (output).
- What role it fills in the system's capability roster.

The SPI validates the manifest at load time. If a plugin's declared reads are within the allowed
surface for its role, it is admitted. If not, it is rejected before it touches production data.

### Why Substrate and Bus are both necessary

Substrate alone gives plugins durable data access, but no live event stream — they can only
react to pre-existing records, not to things happening now.

Bus alone gives plugins live event access, but no durable context — they cannot reason about
history or accumulate their own state.

With both, a plugin gets the full cognitive surface:
- It can read historical context from Substrate before acting.
- It can subscribe to live Pulses that trigger its behavior.
- It can write outputs back to Substrate (within its declared write surface) and publish
  follow-on events to the Bus.

This is the same surface the core system uses. Plugins are not second-class citizens — they are
first-class consumers of the same two fabrics, constrained by their declared manifest.

### Domain profiles as packaging

A domain profile bundles the policy, heuristics, role roster, and gate configuration for a
specific operating context (e.g., "financial trading" or "code review"). Plugins can be
packaged as part of a domain profile, which means adopting a new domain is a single
install operation: load the profile, validate the manifest, and the system operates
in the new context.

---

## What Flows

```
Plugin registration:
  Plugin.manifest → SPI.validate(manifest)
  → if valid: register Plugin in capability roster
  → subscribe Plugin to declared Bus topics
  → grant Substrate read access for declared query surface

Plugin execution (event-driven):
  Bus delivers Pulse(topic ∈ plugin.subscribed_topics) → Plugin.handle(pulse)
  → Plugin reads Substrate(within declared query surface)
  → Plugin produces output
  → Plugin publishes Pulse(topic ∈ plugin.declared_output_topics)
  → Plugin writes to Substrate(within declared write surface)

Plugin teardown:
  Bus publishes lifecycle Pulse(kind=PluginUnloaded) → Plugin.shutdown()
  → Substrate.cleanup(plugin.owned_records) if declared as transient
```

---

## Invariants

1. A plugin cannot read Substrate records outside its declared query surface. The Substrate
   enforcer validates every read at runtime.
2. A plugin cannot publish on a Bus topic not declared in its manifest. The Bus validates
   topic membership at subscribe time.
3. Plugin state (records it writes to Substrate) is tagged with the plugin's identity. If a
   plugin is unloaded, its state is either archived or deleted according to the manifest's
   retention policy.
4. Domain profiles are versioned. A profile upgrade is atomic: the old version is live until the
   new one is fully validated and activated.

---

## Failure Modes

| Failure | Mechanism | Mitigation |
|---|---|---|
| Manifest creep | Plugins request progressively broader Substrate access over versions | Require human approval for any manifest change that expands read or write surface |
| SPI evasion | A plugin calls internal Substrate APIs directly (bypassing the SPI) | Enforce SPI as the only public Substrate interface; make internal APIs private |
| Bus topic pollution | Plugin publishes on undeclared topics by routing through an intermediary | Validate topic authorship at the Bus level, not just at subscribe time |
| Domain profile conflict | Two profiles are active simultaneously with conflicting gate configurations | Allow only one active domain profile per session; profile switching requires explicit teardown |
| Plugin aging | A plugin is loaded but never used; its Bus subscriptions hold open resources indefinitely | Add plugin-inactivity eviction based on demurrage (S1 / S8 patterns) on subscription cost |

---

## Relationship to Other Synergies

- **S1** (Demurrage × HDC): Plugins that produce Engrams are subject to the same demurrage
  pressure as core-produced Engrams. The ecosystem growth path does not exempt plugins from
  memory economics.
- **S3** (c-factor × Bus × HDC): Plugins can contribute new roles and models that diversify
  the agent roster. Their Bus output topics feed the c-factor diversity signal.
- **S10** (TypedContext × domain profiles × Gate): Domain profiles are the same packaging
  mechanism used here. S10 explains how domain profiles interact with the Gate pipeline; S5
  explains how they are installed and activated.

---

## Today vs. Planned

**Today**: Substrate is Shipping; `EventBus<E>` is Built. A basic tool-plugin mechanism exists.
No formalized SPI with declared manifests exists. No domain profile install-and-activate flow
exists.

**Planned**: Plugin SPI ships as a trait + manifest validator. Domain profiles ship as
versioned configuration bundles. Substrate enforces per-plugin read/write surfaces at runtime.
Bus validates topic authorship.

---

## Cross-References

- [`analysis/readiness-audit/subsystem-tools.md`](../readiness-audit/subsystem-tools.md) — tools subsystem readiness and plugin gaps
- [`analysis/readiness-audit/subsystem-agents.md`](../readiness-audit/subsystem-agents.md) — agents subsystem, domain profile integration
- [`analysis/integration-map/agents-x-composition.md`](../integration-map/agents-x-composition.md) — how domain profiles reach the composition layer
- [`analysis/synergy-map/synergy-10-typed-context-domain-safety.md`](synergy-10-typed-context-domain-safety.md) — S10: domain profiles as the safety-audit mechanism
- [`analysis/synergy-map/99-master-synergy-table.md`](99-master-synergy-table.md) — synergy index

---

## Open Questions

- Should domain profiles be installable at runtime (hot-reload) or only at startup? Hot-reload
  is more flexible but raises invariant-safety concerns during profile transition.
- What is the minimum viable manifest schema? Must it enumerate every Substrate table a plugin
  reads, or is a coarser capability set (e.g., "read-only: all Engrams") sufficient?
- How are inter-plugin dependencies expressed? If plugin A depends on plugin B's Substrate
  writes, should the SPI validate this dependency at load time?
