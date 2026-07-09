# Provenance — Taint Flags

> An optional set of flags that mark an Engram as potentially unreliable, without changing its identity.

**Status**: Shipping  
**Crate**: `roko-core`  
**Depends on**: [Overview](00-overview.md), [Trust Level](02-trust-level.md)  
**Used by**: [Score](../score/00-overview.md), [Gate](../../../subsystems/gate/)  
**Last reviewed**: 2026-04-19

---

## TL;DR

`taint` is an `Option<BTreeSet<TaintFlag>>` on the `Provenance` struct. When `None`,
the Engram has no known reliability concerns. When `Some(set)`, the set contains one or
more flags indicating specific reasons to be cautious. Taint flags do **not** change the
Engram's `ContentHash`, so an Engram can be flagged after the fact without creating a
new identity. The Gate pipeline reads taint flags to decide whether to pass or reject an
Engram.

---

## The Idea

Trust level (`TrustLevel`) answers "how well-verified is this Engram?" Taint answers
"are there specific red flags about this Engram?" They are orthogonal:
- A `ChainWitness` Engram can still be tainted (e.g., the chain confirmed its hash, but
  its source was later identified as a hallucinating model).
- A `LocalAgent` Engram with no taint is fine for low-stakes use cases.

Taint is a **set** rather than a single flag so that multiple independent concerns can
coexist. An Engram might be simultaneously flagged as `UnverifiedSource` and `OutdatedAt`
without either flag erasing the other.

---

## Specification

```rust
<!-- source: crates/roko-core/src/provenance.rs -->

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub enum TaintFlag {
    /// The source of the Engram's content is unverified (e.g., external web ingest).
    UnverifiedSource,

    /// The content is suspected to be a model hallucination.
    PossibleHallucination,

    /// The Engram's author was later identified as a compromised agent.
    CompromisedAuthor,

    /// The Engram was produced during an abnormal or adversarial session.
    SuspiciousContext,

    /// The content has been superseded by a newer, authoritative Engram.
    OutdatedAt { superseded_by: ContentHash },

    /// Custom flag for application-specific taint reasons.
    Custom(String),
}
```

---

## Taint in Gate Decisions

The Gate pipeline checks taint before passing an Engram to a consumer:

```rust
<!-- source: crates/roko-core/src/gate.rs -->

fn check_taint(engram: &Engram, policy: &GatePolicy) -> GateVerdict {
    let Some(ref flags) = engram.provenance.taint else {
        return GateVerdict::Pass;
    };
    for flag in flags {
        match flag {
            TaintFlag::PossibleHallucination if policy.reject_hallucinations => {
                return GateVerdict::Reject("hallucination flag".into());
            }
            TaintFlag::CompromisedAuthor => {
                return GateVerdict::Reject("compromised author".into());
            }
            TaintFlag::OutdatedAt { superseded_by } => {
                return GateVerdict::RedirectTo(*superseded_by);
            }
            _ => {}
        }
    }
    GateVerdict::Pass
}
```

---

## Adding Taint Flags

The Substrate exposes a method to add taint flags to an existing Engram without
changing its identity:

```rust
<!-- source: crates/roko-fs/src/substrate.rs -->

/// Add a taint flag to an existing Engram. Does not change ContentHash.
pub fn add_taint(
    &self,
    id: &ContentHash,
    flag: TaintFlag,
    reason: &str,
) -> Result<(), SubstrateError> {
    let mut engram = self.get_mut(id)?;
    engram.provenance.taint
        .get_or_insert_with(BTreeSet::new)
        .insert(flag);
    self.put(engram)?;
    self.audit_log.record_taint(id, flag, reason);
    Ok(())
}
```

Every taint addition is recorded in the audit log with the reason string.

---

## Taint Propagation

When an Engram is derived from a tainted parent (via lineage), the child inherits
a `SuspiciousContext` flag automatically:

```rust
<!-- source: crates/roko-core/src/provenance.rs -->

impl Provenance {
    /// Propagate taint from parent Engrams to a derived child.
    pub fn inherit_taint(parents: &[&Provenance]) -> Option<BTreeSet<TaintFlag>> {
        let inherited: BTreeSet<TaintFlag> = parents.iter()
            .filter(|p| p.taint.is_some())
            .flat_map(|p| p.taint.as_ref().unwrap().iter().cloned())
            .filter(|flag| matches!(flag,
                TaintFlag::CompromisedAuthor |
                TaintFlag::PossibleHallucination
            ))
            .collect();
        if inherited.is_empty() { None } else { Some(inherited) }
    }
}
```

Only `CompromisedAuthor` and `PossibleHallucination` propagate through lineage; other
flags (`UnverifiedSource`, `OutdatedAt`, `SuspiciousContext`, `Custom`) do not, as they
are specific to the Engram where they were set.

<!-- ADDED: propagation semantics — which flags propagate was not explicit in source docs;
inferred from the nature of each flag. -->

---

## Invariants

1. `taint = None` is equivalent to "no taint" — an empty `BTreeSet` must not be stored.
   Use `None` for the untainted case.
2. Taint flags are **not** part of the `ContentHash` — adding a flag does not change identity.
3. Taint can only be **added**, not removed, once set — removal would require creating a
   new Engram with a clean provenance.
4. `CompromisedAuthor` is always a gate-level rejection — it cannot be overridden by policy.
5. Audit log must record every `add_taint` call.

---

## Failure Modes

| Failure | Cause | Recovery |
|---|---|---|
| Taint flag not checked by Gate | Gate policy misconfigured | Policy linter checks that `CompromisedAuthor` is always rejected |
| Inherited taint causes false positives | Parent taint incorrectly propagated | Use conservative propagation (only `CompromisedAuthor`, `PossibleHallucination`) |
| Empty taint set stored | Bug in construction path | Substrate normalizes `Some(empty)` to `None` on write |
| `OutdatedAt` points to missing Engram | Superseding Engram not yet in Substrate | Gate falls back to treating as `Pass` with a warning |

---

## Open Questions

- Should taint flags be revocable (e.g., a flag set in error)? The audit trail makes
  revocation tricky — currently taint is append-only.
- Should `Custom(String)` flags be namespaced like `DecayHandler` names?
  Not currently required.

## See Also

- [`02-trust-level.md`](02-trust-level.md) — complementary reliability signal
- [`05-provenance-propagation.md`](05-provenance-propagation.md) — full propagation rules
- [`../score/00-overview.md`](../score/00-overview.md) — how taint affects scoring
