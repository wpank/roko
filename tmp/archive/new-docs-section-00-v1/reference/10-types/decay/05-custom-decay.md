# Decay — Custom Decay

> A named escape hatch that holds serialized parameters for decay logic not expressible by the built-in variants.

**Status**: Shipping  
**Crate**: `roko-core`  
**Depends on**: [Overview](00-overview.md)  
**Used by**: [Tier Matrix](08-tier-matrix.md)  
**Last reviewed**: 2026-04-19

---

## TL;DR

`Decay::Custom` stores a name string plus an opaque `serde_json::Value` payload. The core
library does not compute a weight for it — that is delegated to a named handler registered
at runtime. This makes it possible to introduce new decay shapes (e.g., logarithmic,
sawtooth, sigmoid) without changing the `Decay` enum. Any Substrate implementation that
encounters an unregistered custom name should fall back to treating the Engram as immortal
rather than silently panicking.

---

## The Idea

Every decay model that can be expressed as "balance as a function of time, parameters, and
retrieval history" can be encoded in the four built-in variants (Demurrage, Exponential,
Step, Linear). Custom exists for the remaining cases:

1. **Experimental models** being prototyped before being promoted to a first-class variant.
2. **Domain-specific schedules** that are application-specific (e.g., a decay that fires
   only during business hours).
3. **Composite models** that chain two decay policies (e.g., exponential decay until
   first retrieval, then demurrage thereafter).
4. **External policy** where the decay curve is computed by a separate service that
   returns a weight on demand.

<!-- ADDED: rationale — Custom is the standard "open-closed principle" escape hatch. New decay
shapes can be iterated quickly without enum churn. The tradeoff is that weight calculation
becomes a runtime dispatch rather than a compile-time match. -->

---

## Specification

```rust
<!-- source: crates/roko-core/src/decay.rs -->

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct CustomDecayParams {
    /// Stable identifier for the decay handler.
    /// Convention: "<crate-name>/<handler-name>", e.g. "roko-neuro/ebbinghaus".
    pub name: String,

    /// Opaque parameters passed to the handler.
    /// Must be serializable; the handler is responsible for deserialization.
    pub params: serde_json::Value,
}
```

The `Decay` enum variant:

```rust
<!-- source: crates/roko-core/src/decay.rs -->

pub enum Decay {
    Demurrage(DemurrageParams),
    Exponential(ExponentialDecayParams),
    Step(StepDecayParams),
    Linear(LinearDecayParams),
    Custom(CustomDecayParams),
}
```

---

## Handler Registration

Custom decay handlers are registered at Substrate construction time:

```rust
<!-- source: crates/roko-core/src/decay.rs -->

/// Trait implemented by custom decay handlers.
pub trait DecayHandler: Send + Sync + 'static {
    /// Return the weight for the given custom params at `now_ms`.
    /// `last_retrieved_ms` is the timestamp of the last successful retrieval (0 if never).
    fn weight_at(
        &self,
        params: &serde_json::Value,
        now_ms: i64,
        created_at_ms: i64,
        last_retrieved_ms: i64,
    ) -> f64;

    /// Update params in place after a retrieval (optional — default is no-op).
    fn on_retrieve(&self, params: &mut serde_json::Value) {}
}
```

The Substrate holds a `HashMap<String, Arc<dyn DecayHandler>>`. When it encounters a
`Decay::Custom(ref p)` it looks up `p.name` in the registry and calls `weight_at`.

**Unregistered handler fallback**: if the name is not in the registry, the Substrate logs a
warning and returns `weight = 1.0` (immortal). This prevents silent data loss when a
handler is not yet loaded.

---

## Naming Convention

Handler names follow the pattern `"<crate-name>/<handler-name>"`:

| Name | Handler |
|---|---|
| `"roko-neuro/ebbinghaus"` | Ebbinghaus forgetting-curve model for consolidated knowledge |
| `"roko-core/sawtooth"` | Sawtooth model that resets on retrieval |
| `"roko-core/sigmoid"` | Sigmoid-shaped decay (slow start, fast middle, slow tail) |

Names must not contain whitespace and are case-sensitive.

---

## Invariants

1. `name` is non-empty.
2. `weight_at` must return a value in `[0.0, 1.0]`. The Substrate clamps to this range
   regardless of what the handler returns.
3. Handler registration is idempotent — registering the same name twice replaces the
   prior handler.
4. An unregistered name **must not** panic — it returns `1.0` (immortal) and emits a
   warning metric.

---

## Failure Modes

| Failure | Cause | Recovery |
|---|---|---|
| Handler not found | Crate that registers handler not loaded | Substrate returns `weight = 1.0`; Engram survives but never decays |
| Handler panics | Bug in custom handler logic | Substrate catches panics, logs, returns `weight = 1.0` |
| Params deserialization fails | Schema mismatch between writer and reader | Handler should return `Err` → Substrate uses fallback weight |
| Name collision | Two crates register same name | Last write wins; emit a startup warning |

---

## Example: Ebbinghaus Handler

The Ebbinghaus forgetting curve is `R = e^(-t/S)` where `t` is time since last review
and `S` is the stability factor (increased with each retrieval). A Custom handler for this:

```rust
<!-- source: crates/roko-neuro/src/ebbinghaus_handler.rs -->

pub struct EbbinghausHandler;

#[derive(Serialize, Deserialize)]
struct EbbinghausParams {
    stability: f64,  // grows with each reinforcement
}

impl DecayHandler for EbbinghausHandler {
    fn weight_at(
        &self,
        params: &serde_json::Value,
        now_ms: i64,
        _created_at_ms: i64,
        last_retrieved_ms: i64,
    ) -> f64 {
        let p: EbbinghausParams = serde_json::from_value(params.clone())
            .unwrap_or(EbbinghausParams { stability: 1.0 });
        let t_days = (now_ms - last_retrieved_ms).max(0) as f64 / 86_400_000.0;
        (-t_days / p.stability).exp().clamp(0.0, 1.0)
    }

    fn on_retrieve(&self, params: &mut serde_json::Value) {
        if let Ok(mut p) = serde_json::from_value::<EbbinghausParams>(params.clone()) {
            p.stability *= 2.0;  // double stability on each successful retrieval
            *params = serde_json::to_value(p).unwrap();
        }
    }
}
```

---

## Interactions

- **Substrate handler registry**: Custom decay is the primary extension point for the
  Substrate's decay subsystem. See `subsystems/substrate/decay-registry.md` (populated
  in a later refactor pass).
- **Serialization**: `serde_json::Value` is the interop type. Custom handlers are responsible
  for versioning their own `params` schema.
- **GC**: The Substrate cannot pre-compute expiry for Custom-decayed Engrams. It must
  call the handler at each compaction pass.

---

## Open Questions

- Should the handler API be async to support external policy services? Currently sync only.
- Should `params` be typed (`Box<dyn Any>`) rather than JSON for in-process handlers?
  The JSON approach was chosen for serialization compatibility; may be revisited.
- Should the fallback weight be `0.0` (treat as expired) rather than `1.0` (immortal)?
  Immortal is safer (no data loss) but can cause substrate bloat if handlers are
  frequently missing.

## See Also

- [`00-overview.md`](00-overview.md) — all decay variants compared
- [`01-demurrage.md`](01-demurrage.md) — primary built-in decay model
- [`06-reinforcement.md`](06-reinforcement.md) — retrieval reinforcement mechanics
