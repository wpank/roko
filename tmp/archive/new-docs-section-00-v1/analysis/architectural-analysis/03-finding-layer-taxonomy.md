# Finding: Five-Layer Taxonomy Coherence

> Dependency audit of all 28 crates. One confirmed violation, six unclassified crates,
> and a VSM mapping that shows the taxonomy is architecturally sound.

**Status**: Analysis — fix needed (F3, F4)
**Crate**: `roko-core`, `roko-conductor`, `roko-learn`
**Depends on**: [Five-Layer Taxonomy](../../reference/08-layers/README.md)
**Last reviewed**: 2026-04-13

---

## TL;DR

The five-layer taxonomy is mostly clean. One confirmed dependency violation
(`roko-conductor` → `roko-learn`) requires a small fix. Six crates need formal layer
assignment. The taxonomy maps cleanly to Beer's Viable System Model.

---

## Dependency Audit

Full Cargo.toml analysis across all 28 crates:

**Clean layers (no violations):**

- **L0** (`roko-core`, `roko-fs`, `roko-std`, `bardo-runtime`, `bardo-primitives`): Zero upward deps.
- **L1** (`roko-agent`, `roko-index`, `roko-lang-*`): Depend only on L0. One dev-dependency exception.
- **L2** (`roko-compose`, `roko-learn`): Depend on L0 and L1. Clean.
- **L4** (`roko-cli`, `roko-orchestrator`): Depend on all layers. Expected for entry points.

**Violations:**

| From | To | Type | Severity |
|---|---|---|---|
| `roko-conductor` (L3/L4) | `roko-learn` (L2/Cross-cut) | Direct compile-time dependency | **Medium** |
| `roko-agent` (L1) | `roko-learn` (L2/Cross-cut) | Dev-dependency only | **Low** |

---

## The roko-conductor Violation (F3)

**Root cause**: `roko-conductor` imports learning types for circuit breaker state tracking.
The Conductor needs to know about historical failure rates (a learning concern) to make
circuit breaker decisions (a harness concern).

**Fix**: Extract a `HealthMetrics` trait into `roko-core` (L0):

```rust
// In roko-core/src/traits.rs
pub trait HealthMetrics: Send + Sync {
    fn failure_rate(&self, gate: &str, window: Duration) -> f32;
    fn avg_latency(&self, gate: &str, window: Duration) -> Duration;
}
```

Then `roko-conductor` depends on `&dyn HealthMetrics` (L0 trait), and `roko-learn` implements
it. The dependency flows downward. This is the canonical pattern for resolving upward
dependencies in layered architectures.

**Impact of fix**: Restores clean L3→L0 dependency. No behavior change.

---

## Unclassified Crates (F4)

Six crates need formal layer assignment:

| Crate | Recommended Classification | Rationale |
|---|---|---|
| `roko-neuro` | **Cross-cut** | Bridges L0-L2 for knowledge; inject via `&dyn Substrate` |
| `roko-daimon` | **Cross-cut** | No upward deps (only roko-core); inject via PAD trait object |
| `roko-dreams` | **Cross-cut** | Bridges Neuro + Daimon at Delta frequency |
| `roko-golem` | **Phase 2+ umbrella** | Contains Daimon and Dreams code pending dissolution |
| `roko-chain` | **L1 Domain Plugin** | Analogous to roko-agent for chain domain |
| `roko-plugin` | **L1 Framework** | Plugin SDK extending the tool/agent system |

---

## roko-fs Layer Assignment Error (F5/D2)

`roko-fs` is listed under L3 Harness in the layer taxonomy documentation (line 116 of
`12-five-layer-taxonomy.md`: "roko-fs — JSONL substrate persistence, garbage collection,
file layout") but functionally it is an L0 Runtime crate. It implements `FileSubstrate`,
which is a `Substrate` trait implementation — and Substrate is assigned to L0.

**Resolution**: Move `roko-fs` to L0 Runtime in the documentation. Its sole purpose is
persistent storage of Engrams, which is the canonical L0 responsibility.

---

## VSM Mapping Completeness

The five layers map cleanly to Beer's Viable System Model:

| VSM System | Layer | Function | Clean? |
|---|---|---|---|
| System 1 (Operations) | L0 Runtime | Process lifecycle, I/O | Yes |
| System 2 (Coordination) | L1 Framework | Prevent conflict between agents | Yes |
| System 3 (Control) | L2 Scaffold | Optimize resource allocation (context, tokens) | Yes |
| System 3* (Audit) | L3 Harness | Verify quality | Yes |
| System 4 (Intelligence) | L4 Orchestration | Plan, adapt, look forward | Yes |
| System 5 (Policy) | L4 + Daimon | Identity, self-model | Partially — spans L4 and a cross-cut |

The only imperfect mapping is System 5 (Policy/Identity), which spans both the L4 orchestration
layer and the Daimon cross-cut. This is acceptable because Beer's VSM explicitly allows System 5
to draw from multiple subsystems — it is the meta-system that integrates all others.

---

## Related Findings

- [F10 — Cross-Cut Isolation](06-finding-crosscut-isolation.md): The classification of Neuro,
  Daimon, Dreams as cross-cuts is the formal grounding for their injection pattern.
- [Integration Map — architecture×conductor](../integration-map/architecture-x-conductor.md):
  The conductor dependency violation is specifically about this pair.

## References

- Beer, S. (1972). "Brain of the Firm." Allen Lane.

## Open Questions

- Should `roko-golem` be dissolved before Phase 2, or is it acceptable as a migration umbrella
  through Phase 1?
- Is the `roko-agent` dev-dependency on `roko-learn` (L2) acceptable long-term, or should it
  be refactored away?
